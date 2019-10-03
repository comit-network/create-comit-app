#[macro_export]
macro_rules! print_progress {
    ($($arg:tt)*) => ({
        print!($($arg)*);
        print!("...");
        std::io::stdout().flush().ok().expect("Could not flush stdout");
    })
}
