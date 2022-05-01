use {
    argparse::{ArgumentParser, Store},
    chrono::prelude::{Local, Timelike},
    once_cell::sync::Lazy,
    openweathermap::blocking::weather,
    std::{env, fs::File, io::Read, process},
    subprocess::{Exec, Pipeline, Redirection},
    substring::Substring,
    systemstat::{saturating_sub_bytes, Platform, System},
    unicode_segmentation::UnicodeSegmentation,
};

#[derive(Debug)]
enum CommandKind {
    Pacman,
    Apt,
    Xbps,
    Portage,
    Apk,
    Dnf,
}

static JSON: Lazy<serde_json::Value> = Lazy::new(read_config);

fn read_config() -> serde_json::Value {
    let mut path = format!("{}/.config/hello-rs/config.json", env::var("HOME").unwrap());
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("A simple greeter for your terminal, made in Rust");
        ap.refer(&mut path).add_option(
            &["-c", "--config"],
            Store,
            "Specify a path to a config file",
        );
        ap.parse_args_or_exit();
    }
    serde_json::from_reader(File::open(path).expect("Failed to open config file."))
        .expect("Failed to parse config file as a JSON.")
}

fn update_commmand(command: String) -> (CommandKind, Pipeline) {
    match command.trim_matches('\"') {
        "pacman" => (
            CommandKind::Pacman,
            Exec::cmd("checkupdates") | Exec::cmd("wc").arg("-l"),
        ),
        "apt" => (
            CommandKind::Apt,
            Exec::cmd("apt")
                .args(&["list", "-u"])
                .stderr(Redirection::File(File::open("/dev/null").unwrap()))
                | Exec::cmd("tail").args(&["-n", "+2"])
                | Exec::cmd("wc").arg("-l"),
        ),
        "xbps" => (CommandKind::Xbps, {
            Exec::cmd("xbps-install").arg("-Sun") | Exec::cmd("wc").arg("-l")
        }),
        "portage" => (
            CommandKind::Portage,
            Exec::cmd("eix").args(&["-u", "--format", "'<installedversions:nameversion>'"])
                | Exec::cmd("tail").arg("-1")
                | Exec::cmd("cut").args(&["-d", " ", "-f2"]),
        ),
        "apk" => (CommandKind::Apk, {
            Exec::cmd("apk").args(&["-u", "list"]) | Exec::cmd("wc").arg("-l")
        }),
        "dnf" => (
            CommandKind::Dnf,
            Exec::cmd("dnf").arg("check-update")
                | Exec::cmd("tail").args(&["-n", "+3"])
                | Exec::cmd("wc").arg("-l"),
        ),
        other => panic!("Unsupported package manager: {}", other),
    }
}

fn check_updates() -> i32 {
    let mut total_updates = 0;
    let mut commands = Vec::new();

    if JSON["package_managers"] == serde_json::json![null] {
        return -1;
    }

    if JSON["package_managers"].is_array() {
        let pm = JSON["package_managers"].as_array().unwrap();

        for arg in pm {
            let (kind, exec) = update_commmand(arg.to_string());
            let reader = exec.stream_stdout().unwrap();
            commands.push((kind, reader));
        }
    } else {
        let pm = &JSON["package_managers"];
        let (kind, exec) = update_commmand(pm.to_string());
        let reader = exec.stream_stdout().unwrap();
        commands.push((kind, reader));
    }

    for (kind, mut reader) in commands {
        let mut s = String::new();
        reader.read_to_string(&mut s).unwrap(); // this part definitely blocks

        match kind {
            CommandKind::Portage => {
                if s.trim_end_matches('\n') != "matches" {
                    total_updates += s.trim_end_matches('\n').parse::<i32>().unwrap_or(1);
                }
            }
            _ => {
                total_updates += s.trim_end_matches('\n').parse::<i32>().unwrap();
            }
        }
    }

    total_updates
}

fn count_command(command: String) -> Pipeline {
    match command.trim_matches('\"') {
        "pacman" => Exec::cmd("pacman").arg("-Q") | Exec::cmd("wc").arg("-l"),
        "apt" => {
            Exec::cmd("dpkg-query").arg("-l")
                | Exec::cmd("grep").arg("ii")
                | Exec::cmd("wc").arg("-l")
        }
        "xbps" => Exec::cmd("xbps-query").arg("-l") | Exec::cmd("wc").arg("-l"),
        "portage" => Exec::cmd("eix-installed").arg("-a") | Exec::cmd("wc").arg("-l"),
        "apk" => Exec::cmd("apk").arg("info") | Exec::cmd("wc").arg("-l"),
        "dnf" => {
            Exec::cmd("dnf").args(&["list", "installed"])
                | Exec::cmd("tail").args(&["-n", "+2"])
                | Exec::cmd("wc").arg("-l")
        }
        other => panic!("unknown package manager: {}", other),
    }
}

fn get_package_count() -> i32 {
    let mut total_packages = 0;
    let mut commands = Vec::new();

    if JSON["package_managers"] == serde_json::json![null] {
        return -1;
    }

    if JSON["package_managers"].is_array() {
        let pm = JSON["package_managers"].as_array().unwrap();
        for arg in pm {
            commands.push(count_command(arg.to_string()).stream_stdout().unwrap());
        }
    } else {
        let pm = &JSON["package_managers"];
        commands.push(count_command(pm[0].to_string()).stream_stdout().unwrap());
    }

    for mut reader in commands {
        let mut s = String::new();
        reader.read_to_string(&mut s).unwrap(); // this part definitely blocks

        total_packages += s.trim_end_matches('\n').parse::<i32>().unwrap();
    }

    total_packages
}

fn get_release() -> String {
    let rel = Exec::cmd("lsb_release")
        .args(&["-s", "-d"])
        .capture()
        .unwrap()
        .stdout_str();
    if rel.len() > 41 {
        format!("{}...", rel.trim_matches('\"').substring(0, 37))
    } else {
        rel.trim_matches('\"')
            .trim_end_matches('\n')
            .trim_end_matches('\"')
            .to_string()
    }
}

fn get_kernel() -> String {
    let uname = Exec::cmd("uname")
        .arg("-sr")
        .capture()
        .unwrap()
        .stdout_str();
    if uname.len() > 41 {
        format!("{}...", uname.substring(0, 37))
    } else {
        uname.trim_end_matches('\n').to_string()
    }
}

fn get_song() -> String {
    if JSON["song"] == false {
        return "".to_string();
    }
    let song = process::Command::new("playerctl")
        .arg("metadata")
        .arg("-f")
        .arg("{{ artist }} - {{ title }}")
        .output()
        .unwrap();
    let songerr = String::from_utf8_lossy(&song.stderr);
    let songname = String::from_utf8_lossy(&song.stdout);
    if songerr != "No players found" {
        if songname.len() > 41 {
            format!("{}...", songname.substring(0, 37))
        } else {
            songname.trim_end_matches('\n').to_string()
        }
    } else {
        "".to_string()
    }
}

fn upper_first(s: String) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn calc_whitespace(text: String) -> String {
    let size = 45 - text.graphemes(true).count();
    let final_string = format!("{}{}", " ".repeat(size), "│");
    format!("{}{}", text, final_string)
}

fn calc_with_hostname(text: String) -> String {
    let size = 55 - text.graphemes(true).count();
    let final_string = format!("{}{}", "─".repeat(size), "╮");
    format!("{}{}", text, final_string)
}

fn get_environment() -> String {
    env::var::<String>(ToString::to_string(&"XDG_CURRENT_DESKTOP"))
        .unwrap_or_else(|_| env::var(&"XDG_SESSION_DESKTOP").unwrap_or_else(|_| "".to_string()))
}

fn get_weather() -> String {
    let deg;
    let icon_code;
    let icon;
    let main;
    let temp;
    let location = JSON
        .get("location")
        .expect("Couldn't find 'location' attribute.")
        .to_string();
    let units = JSON
        .get("units")
        .expect("Couldn't find 'units' attribute.")
        .to_string();
    let lang = JSON
        .get("lang")
        .expect("Couldn't find 'lang' attribute.")
        .to_string();
    let api_key = JSON
        .get("api_key")
        .expect("Couldn't find 'api_key' attribute.")
        .to_string();
    match &weather(
        location.trim_matches('\"'),
        units.trim_matches('\"'),
        lang.trim_matches('\"'),
        api_key.trim_matches('\"'),
    ) {
        Ok(current) => {
            deg = if units.trim_matches('\"') == "imperial" {
                "F"
            } else {
                "C"
            };
            icon_code = &current.weather[0].icon;
            icon = match icon_code.as_ref() {
                "01d" => "☀️",
                "01n" => "🌙",
                "02d" => "⛅️",
                "02n" => "🌙",
                "03d" => "☁️",
                "03n" => "☁️",
                "04d" => "☁️",
                "04n" => "☁️",
                "09d" => "🌧️",
                "09n" => "🌧️",
                "10d" => "🌧️",
                "10n" => "🌧️",
                "11d" => "⛈️",
                "11n" => "⛈️",
                "13d" => "🌨️",
                "13n" => "🌨️",
                "40d" => "🌫️",
                "40n" => "🌫️",
                "50d" => "🌫️",
                "50n" => "🌫️",
                _ => "❓",
            };
            main = current.weather[0].main.to_string();
            temp = current.main.temp.to_string();
        }
        Err(e) => panic!("Could not fetch weather because: {}", e),
    }
    format!("│ {} {} {}°{}", icon, main, temp.substring(0, 2), deg)
}

fn greeting() -> String {
    let name = JSON
        .get("name")
        .expect("Couldn't find 'name' attribute.")
        .to_string();
    match Local::now().hour() {
        6..=11 => "🌇 Good morning",
        12..=17 => "🏙️ Good afternoon",
        18..=22 => "🌆 Good evening",
        _ => "🌃 Good night",
    }
    .to_string()
        + ", "
        + name.trim_matches('\"')
}

fn get_hostname() -> String {
    JSON.get("hostname")
        .expect("Couldn't find 'hostname' attribute.")
        .to_string()
        .trim_matches('\"')
        .to_string()
}

fn get_datetime() -> String {
    let dt = Local::now();
    let day = dt.format("%e").to_string();
    let date = match day.trim_start_matches(' ') {
        "1" | "21" | "31 " => format!("{} {}st", dt.format("%B"), day.trim_start_matches(' ')),
        "2" | "22" => format!("{} {}nd", dt.format("%B"), day.trim_start_matches(' ')),
        "3" | "23" => format!("{} {}rd", dt.format("%B"), day.trim_start_matches(' ')),
        _ => format!("{} {}th", dt.format("%B"), day.trim_start_matches(' ')),
    };
    let time = match JSON
        .get("time_format")
        .expect("Couldn't find 'time_format' attribute.")
        .to_string()
        .trim_matches('\"')
    {
        "12h" => dt.format("%l:%M %p").to_string(),
        "24h" => dt.format("%H:%M").to_string(),
        _ => "off".to_string(),
    };
    let time_icon = match dt.hour() {
        0 | 12 => "🕛",
        1 | 13 => "🕐",
        2 | 14 => "🕑",
        3 | 15 => "🕒",
        4 | 16 => "🕓",
        5 | 17 => "🕔",
        6 | 18 => "🕕",
        7 | 19 => "🕖",
        8 | 20 => "🕗",
        9 | 21 => "🕘",
        10 | 22 => "🕙",
        11 | 23 => "🕚",
        _ => "🕛",
    };
    format!("│ {} {}, {}", time_icon, date, time.trim_start_matches(' '))
}

fn count_updates() -> String {
    let count = check_updates();
    let update_count;
    let updates: String = match count {
        -1 => "none",
        0 => "☑️ Up to date",
        1 => "1️⃣ 1 update",
        2 => "2️⃣ 2 updates",
        3 => "3️⃣ 3 updates",
        4 => "4️⃣ 4 updates",
        5 => "5️⃣ 5 updates",
        6 => "6️⃣ 6 updates",
        7 => "7️⃣ 7 updates",
        8 => "8️⃣ 8 updates",
        9 => "9️⃣ 9 updates",
        10 => "🔟 10 updates",
        _ => {
            update_count = format!("‼️ {} updates", count);
            update_count.as_ref()
        }
    }
    .to_string();
    format!("│ {}", updates)
}

fn get_memory() -> String {
    match System::new().memory() {
        Ok(mem) => format!("{} Used", saturating_sub_bytes(mem.total, mem.free)),
        Err(x) => panic!("Could not get memory because: {}", x),
    }
}

fn get_disk_usage() -> String {
    match System::new().mount_at("/") {
        Ok(disk) => {
            format!("{} Free", disk.free)
        }
        Err(x) => panic!("Could not get disk usage because: {}", x),
    }
}

fn main() {
    let hostname = std::thread::spawn(get_hostname);
    let greeting = std::thread::spawn(greeting);
    let datetime = std::thread::spawn(get_datetime);
    let weather = std::thread::spawn(get_weather);
    let release = std::thread::spawn(get_release);
    let kernel = std::thread::spawn(get_kernel);
    let memory = std::thread::spawn(get_memory);
    let disk = std::thread::spawn(get_disk_usage);
    let environment = std::thread::spawn(get_environment);
    let up_count = std::thread::spawn(count_updates);
    let package_count = std::thread::spawn(get_package_count);
    let song = std::thread::spawn(get_song);

    println!(
        "{}",
        calc_with_hostname(format!("╭─\x1b[32m{}\x1b[0m", hostname.join().unwrap()))
    );

    println!(
        "{}",
        calc_whitespace(format!("│ {}!", greeting.join().unwrap()))
    );
    println!("{}", calc_whitespace(datetime.join().unwrap()));
    println!("{}", calc_whitespace(weather.join().unwrap()));
    println!(
        "{}",
        calc_whitespace(format!("│ 💻 {}", release.join().unwrap()))
    );
    println!(
        "{}",
        calc_whitespace(format!("│ 🫀 {}", kernel.join().unwrap()))
    );
    println!(
        "{}",
        calc_whitespace(format!("│ 🧠 {}", memory.join().unwrap()))
    );
    println!(
        "{}",
        calc_whitespace(format!("│ 💾 {}", disk.join().unwrap()))
    );

    let environment = environment.join().unwrap();
    match environment.as_ref() {
        "" => (),
        _ => println!(
            "{}",
            calc_whitespace(format!("│ 🖥️ {}", upper_first(environment)))
        ),
    }

    let up_count = up_count.join().unwrap();
    if up_count != *"│ none".to_string() {
        println!("{}", calc_whitespace(up_count));
    }

    let package_count = package_count.join().unwrap();
    match package_count {
        -1 => (),
        0 => println!("{}", calc_whitespace("│ 📦 No packages".to_string())),
        1 => println!("{}", calc_whitespace("│ 📦 1 package".to_string())),
        _ => println!(
            "{}",
            calc_whitespace(format!("│ 📦 {} packages", package_count))
        ),
    }

    let song = song.join().unwrap();
    match song.as_ref() {
        "" => (),
        _ => println!(
            "{}",
            calc_whitespace(format!("│ 🎵 {}", song.trim_matches('\n')))
        ),
    }

    println!("╰─────────────────────────────────────────────╯");
}
