pub fn log_no_event(msg: std::fmt::Arguments) {
    println!("No event performed. {}", msg);
}

pub fn log_event(msg: std::fmt::Arguments) {
    println!("Event performed. {}", msg);
}

pub fn trace<T:std::fmt::Debug>(x : T) -> T{
    println!("{:?}", x);
    x
}

#[macro_export]
macro_rules! log_no_event {
    ($msg:expr $(, $args:expr)*) => {
        $crate::util::log_no_event(format_args!($msg $(, $args)*))
    };
}
#[macro_export]
macro_rules! log_event {
    ($msg:expr $(, $args:expr)*) => {
        $crate::util::log_event(format_args!($msg $(, $args)*))
    };
}