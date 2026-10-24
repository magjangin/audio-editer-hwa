pub fn format_timestamp(sec: f32) -> String {
    let sec = sec.max(0.0);
    let mins = (sec / 60.0).floor() as u32;
    let rem_sec = sec % 60.0;
    format!("{:02}:{:05.2}", mins, rem_sec)
}

pub fn format_ms_timestamp(sec: f32) -> String {
    let sec = sec.max(0.0);
    let mins = (sec / 60.0).floor() as u32;
    let rem_sec = sec % 60.0;
    format!("{:02}:{:07.4}", mins, rem_sec)
}

pub fn format_precise_timestamp(sec: f32) -> String {
    let sec = sec.max(0.0);
    let hours = (sec / 3600.0).floor() as u32;
    let mins = ((sec % 3600.0) / 60.0).floor() as u32;
    let rem_sec = sec % 60.0;
    let subsec_4 = ((rem_sec.fract() * 10000.0).round() as u32) % 10000;
    format!("{:02}:{:02}:{:02}.{:04}", hours, mins, rem_sec.floor() as u32, subsec_4)
}
