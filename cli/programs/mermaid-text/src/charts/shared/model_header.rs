pub fn body<'a>(source: &'a str, header: &str) -> Result<std::str::Lines<'a>, String> {
    let mut lines = source.lines();
    if lines.next().unwrap_or("").trim() != header {
        return Err("expected usecase-beta or wardley-beta on a line by itself".into());
    }
    Ok(lines)
}
