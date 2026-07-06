#[macro_export]
macro_rules! env_int {
    ($var:expr) => {
        match std::env::var($var) {
            Ok(val) => val.parse(),
            Err(err) => Err(err)
        }
    };
    ($var:expr, $default:expr) => {
        std::env::var($var).map(|e| e.parse::<i32>().unwrap_or($default)).unwrap_or($default)
    };
}
#[macro_export]
macro_rules! env_str {
    ($var:expr) => {
        std::env::var($var)
    };
    ($var:expr, $default:expr) => {
        std::env::var($var).unwrap_or($default.to_string())
    }
}
