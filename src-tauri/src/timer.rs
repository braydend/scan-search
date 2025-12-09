use std::time::SystemTime;

pub fn timer<T>(label: &str, func: impl FnOnce()->T) -> rusqlite::Result<T> {
    let start = SystemTime::now();
    let result = func();
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();
    println!("{} complete in ({:?})", label, duration);
    Ok(result)
}