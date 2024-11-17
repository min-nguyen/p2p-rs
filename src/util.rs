pub fn update(msg: std::fmt::Arguments) {
    println!("[Internal update]:\n{}", msg);
}

pub fn received(msg: std::fmt::Arguments) {
    println!("[Received message]:\n{}", msg);
}

pub fn responded(msg: std::fmt::Arguments) {
    println!("[Broadcasted message]:\n{}", msg);
}

pub fn trace<T: std::fmt::Debug>(x: T) -> T {
    println!("{:?}", x);
    x
}

pub fn abbrev(hex: &str) -> String {
    let mut s: String = hex.to_owned();
    if hex.len() > 20 {
        s.truncate(16);
        s.push_str("...");
    }
    s
}

#[macro_export]
macro_rules! update {
    ($msg:expr $(, $args:expr)*) => {
        $crate::util::update(format_args!($msg $(, $args)*))
    };
}
#[macro_export]
macro_rules! received {
    ($msg:expr $(, $args:expr)*) => {
        $crate::util::received(format_args!($msg $(, $args)*))
    };
}
#[macro_export]
macro_rules! responded {
    ($msg:expr $(, $args:expr)*) => {
        $crate::util::responded(format_args!($msg $(, $args)*))
    };
}
