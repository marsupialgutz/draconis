use systemstat::{System, Platform, saturating_sub_bytes};

use {
    argparse::{ArgumentParser, Store},
    chrono::prelude::{Local, Timelike},
    openweathermap::blocking::weather,
    std::{env, fs, process},
    subprocess::Exec,
    substring::Substring,
    unicode_segmentation::UnicodeSegmentation,
};

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
    serde_json::from_reader(fs::File::open(path).expect("Failed to open config file."))
        .expect("Failed to parse config file as a JSON.")
}

fn check_updates() -> i32 {
    let mut total_updates = 0;

    let json = read_config();

    if json["package_managers"] == serde_json::json![null] {
        return -1;
    }

    if json["package_managers"].is_array() {
        let pm = json["package_managers"].as_array().unwrap();
        (0..pm.len()).for_each(|i| match pm[i].to_string().trim_matches('\"') {
            "pacman" => {
                let update_count = { Exec::cmd("checkupdates") | Exec::cmd("wc").arg("-l") }
                    .capture()
                    .unwrap()
                    .stdout_str();
                total_updates += update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "apt" => {
                let update_count = {
                    Exec::cmd("apt").arg("list").arg("-u")
                        | Exec::cmd("tail").arg("-n").arg("+2")
                        | Exec::cmd("wc").arg("-l")
                }
                .capture()
                .unwrap()
                .stdout_str();
                total_updates += update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "xbps" => {
                let update_count =
                    { Exec::cmd("xbps-install").arg("-Sun") | Exec::cmd("wc").arg("-l") }
                        .capture()
                        .unwrap()
                        .stdout_str();
                total_updates += update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "portage" => {
                let update_count = {
                    Exec::cmd("eix")
                        .arg("-u")
                        .arg("--format")
                        .arg("'<installedversions:nameversion>'")
                        | Exec::cmd("tail").arg("-1")
                        | Exec::cmd("cut").arg("-d").arg(" ").arg("-f2")
                }
                .capture()
                .unwrap()
                .stdout_str();
                if update_count.trim_end_matches('\n') != "matches" {
                    total_updates += update_count
                        .trim_end_matches('\n')
                        .parse::<i32>()
                        .unwrap_or(1);
                }
            }
            "apk" => {
                let update_count =
                    { Exec::cmd("apk").arg("-u").arg("list") | Exec::cmd("wc").arg("-l") }
                        .capture()
                        .unwrap()
                        .stdout_str();
                total_updates += update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "dnf" => {
                let update_count = {
                    Exec::cmd("dnf").arg("check-update")
                        | Exec::cmd("tail").arg("-n").arg("+3")
                        | Exec::cmd("wc").arg("-l")
                }
                .capture()
                .unwrap()
                .stdout_str();
                total_updates += update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            _ => (),
        });
    } else {
        let pm = &json["package_managers"];
        match pm.to_string().trim_matches('\"') {
            "pacman" => {
                let update_count = { Exec::cmd("checkupdates") | Exec::cmd("wc").arg("-l") }
                    .capture()
                    .unwrap()
                    .stdout_str();
                total_updates = update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "apt" => {
                let update_count = {
                    Exec::cmd("apt").arg("list").arg("-u")
                        | Exec::cmd("tail").arg("-n").arg("+2")
                        | Exec::cmd("wc").arg("-l")
                }
                .capture()
                .unwrap()
                .stdout_str();
                total_updates = update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "xbps" => {
                let update_count =
                    { Exec::cmd("xbps-install").arg("-Sun") | Exec::cmd("wc").arg("-l") }
                        .capture()
                        .unwrap()
                        .stdout_str();
                total_updates = update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "portage" => {
                let update_count = {
                    Exec::cmd("eix")
                        .arg("-u")
                        .arg("--format")
                        .arg("<installedversions:nameversion>")
                        | Exec::cmd("tail").arg("-1")
                        | Exec::cmd("cut").arg("-d").arg(" ").arg("-f2")
                }
                .capture()
                .unwrap()
                .stdout_str();
                if update_count != "matches" {
                    total_updates = update_count
                        .trim_end_matches('\n')
                        .parse::<i32>()
                        .unwrap_or(1);
                }
            }
            "apk" => {
                let update_count =
                    { Exec::cmd("apk").arg("-u").arg("list") | Exec::cmd("wc").arg("-l") }
                        .capture()
                        .unwrap()
                        .stdout_str();
                total_updates = update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "dnf" => {
                let update_count = {
                    Exec::cmd("dnf").arg("check-update")
                        | Exec::cmd("tail").arg("-n").arg("+3")
                        | Exec::cmd("wc").arg("-l")
                }
                .capture()
                .unwrap()
                .stdout_str();
                total_updates = update_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            _ => (),
        }
    }

    total_updates
}

fn get_package_count() -> i32 {
    let mut total_packages = 0;

    let json = read_config();

    if json["package_managers"] == serde_json::json![null] {
        return -1;
    }

    if json["package_managers"].is_array() {
        let pm = json["package_managers"].as_array().unwrap();
        (0..pm.len()).for_each(|i| match pm[i].to_string().trim_matches('\"') {
            "pacman" => {
                let package_count = { Exec::cmd("pacman").arg("-Q") | Exec::cmd("wc").arg("-l") }
                    .capture()
                    .unwrap()
                    .stdout_str();
                total_packages += package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "apt" => {
                let package_count = {
                    Exec::cmd("dpkg-query").arg("-l")
                        | Exec::cmd("grep").arg("ii")
                        | Exec::cmd("wc").arg("-l")
                }
                .capture()
                .unwrap()
                .stdout_str();
                total_packages += package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "xbps" => {
                let package_count =
                    { Exec::cmd("xbps-query").arg("-l") | Exec::cmd("wc").arg("-l") }
                        .capture()
                        .unwrap()
                        .stdout_str();
                total_packages += package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "portage" => {
                let package_count =
                    { Exec::cmd("eix-installed").arg("-a") | Exec::cmd("wc").arg("-l") }
                        .capture()
                        .unwrap()
                        .stdout_str();
                total_packages += package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "apk" => {
                let package_count = { Exec::cmd("apk").arg("info") | Exec::cmd("wc").arg("-l") }
                    .capture()
                    .unwrap()
                    .stdout_str();
                total_packages += package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "dnf" => {
                let package_count = {
                    Exec::cmd("dnf").arg("list").arg("installed")
                        | Exec::cmd("tail").arg("-n").arg("+2")
                        | Exec::cmd("wc").arg("-l")
                }
                .capture()
                .unwrap()
                .stdout_str();
                total_packages += package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            _ => (),
        });
    } else {
        let pm = &json["package_managers"];
        match pm[0].to_string().trim_matches('\"') {
            "pacman" => {
                let package_count = { Exec::cmd("pacman").arg("-Q") | Exec::cmd("wc").arg("-l") }
                    .capture()
                    .unwrap()
                    .stdout_str();
                total_packages = package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "apt" => {
                let package_count = {
                    Exec::cmd("dpkg-query").arg("-l")
                        | Exec::cmd("grep").arg("ii")
                        | Exec::cmd("wc").arg("-l")
                }
                .capture()
                .unwrap()
                .stdout_str();
                total_packages = package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "xbps" => {
                let package_count =
                    { Exec::cmd("xbps-query").arg("-l") | Exec::cmd("wc").arg("-l") }
                        .capture()
                        .unwrap()
                        .stdout_str();
                total_packages = package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "portage" => {
                let package_count =
                    { Exec::cmd("eix-installed").arg("-a") | Exec::cmd("wc").arg("-l") }
                        .capture()
                        .unwrap()
                        .stdout_str();
                total_packages = package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "apk" => {
                let package_count = { Exec::cmd("apk").arg("info") | Exec::cmd("wc").arg("-l") }
                    .capture()
                    .unwrap()
                    .stdout_str();
                total_packages = package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            "dnf" => {
                let package_count = {
                    Exec::cmd("dnf").arg("list").arg("installed")
                        | Exec::cmd("tail").arg("-n").arg("+2")
                        | Exec::cmd("wc").arg("-l")
                }
                .capture()
                .unwrap()
                .stdout_str();
                total_packages = package_count.trim_end_matches('\n').parse::<i32>().unwrap();
            }
            _ => (),
        }
    }

    total_packages
}

fn get_release() -> String {
    let rel = Exec::cmd("lsb_release")
        .arg("-s")
        .arg("-d")
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
    let json = read_config();
    if json["song"] == false {
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
        .unwrap_or_else(|_| "".to_string())
}

fn get_weather() -> String {
    let deg;
    let icon_code;
    let icon;
    let main;
    let temp;
    let json = read_config();
    let location = json
        .get("location")
        .expect("Couldn't find 'location' attribute.")
        .to_string();
    let units = json
        .get("units")
        .expect("Couldn't find 'units' attribute.")
        .to_string();
    let lang = json
        .get("lang")
        .expect("Couldn't find 'lang' attribute.")
        .to_string();
    let api_key = json
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
    let dt = Local::now();
    let json = read_config();
    let name = json
        .get("name")
        .expect("Couldn't find 'name' attribute.")
        .to_string();
    match dt.hour() {
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
    let json = read_config();
    json.get("hostname")
        .expect("Couldn't find 'hostname' attribute.")
        .to_string()
        .trim_matches('\"')
        .to_string()
}

fn get_datetime() -> String {
    let time_icon;
    let json = read_config();
    let time_format = json
        .get("time_format")
        .expect("Couldn't find 'time_format' attribute.")
        .to_string();
    let dt = Local::now();
    let day = dt.format("%e").to_string();
    let date = match day.trim_start_matches(' ') {
        "1" | "21" | "31 " => format!("{} {}st", dt.format("%B"), day.trim_start_matches(' ')),
        "2" | "22" => format!("{} {}nd", dt.format("%B"), day.trim_start_matches(' ')),
        "3" | "23" => format!("{} {}rd", dt.format("%B"), day.trim_start_matches(' ')),
        _ => format!("{} {}th", dt.format("%B"), day.trim_start_matches(' ')),
    };
    let time = match time_format.trim_matches('\"') {
        "12h" => dt.format("%l:%M %p").to_string(),
        "24h" => dt.format("%H:%M").to_string(),
        _ => "off".to_string(),
    };
    time_icon = match dt.hour() {
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

fn get_cpu() -> String {
    let sys = System::new();
    match sys.load_average() {
        Ok(loadavg) => {
            format!("{}% Used", (loadavg.one * 10.0) as u32)
        }
        Err(x) => panic!("Could not get CPU load: {}", x),
    }
}

fn get_memory() -> String {
    let sys = System::new();
    match sys.memory() {
        Ok(mem) => {
            format!("{} Used", saturating_sub_bytes(mem.total, mem.free)).to_string()
        }
        Err(x) => panic!("Could not get memory because: {}", x),
    }
}

fn get_disk_usage() -> String {
    let sys = System::new();
    match sys.mount_at("/") {
        Ok(disk) => {
            format!("{} Free", disk.free.to_string())
        }
        Err(x) => panic!("Could not get disk usage because: {}", x),
    }
}

fn main() {
    println!(
        "{}",
        calc_with_hostname(format!("╭─\x1b[32m{}\x1b[0m", get_hostname()))
    );
    println!("{}", calc_whitespace(format!("│ {}!", greeting())));
    println!("{}", calc_whitespace(get_datetime()));
    println!("{}", calc_whitespace(get_weather()));
    println!("{}", calc_whitespace(format!("│ 💻 {}", get_release())));
    println!("{}", calc_whitespace(format!("│ 🫀 {}", get_kernel())));
    println!("{}", calc_whitespace(format!("│ 🔌 {}", get_cpu())));
    println!("{}", calc_whitespace(format!("│ 🧠 {}", get_memory())));
    println!("{}", calc_whitespace(format!("│ 💾 {}", get_disk_usage())));

    match get_environment().as_ref() {
        "" => (),
        _ => println!(
            "{}",
            calc_whitespace(format!("│ 🖥️ {}", upper_first(get_environment())))
        ),
    }

    if count_updates() != "│ none".to_string() {
        println!("{}", calc_whitespace(count_updates()));
    }

    match get_package_count() {
        -1 => (),
        0 => println!("{}", calc_whitespace("│ 📦 No packages".to_string())),
        1 => println!("{}", calc_whitespace("│ 📦 1 package".to_string())),
        _ => println!(
            "{}",
            calc_whitespace(format!("│ 📦 {} packages", get_package_count()))
        ),
    }

    match get_song().as_ref() {
        "" => (),
        _ => println!(
            "{}",
            calc_whitespace(format!("│ 🎵 {}", get_song().trim_matches('\n')))
        ),
    }

    println!("╰─────────────────────────────────────────────╯");
}
