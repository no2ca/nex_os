use core::sync::atomic::{AtomicU8, Ordering};

#[allow(unused)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);

#[inline]
pub fn set_log_level(level: LogLevel) {
    LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

#[inline]
pub fn log_enabled(level: LogLevel) -> bool {
    (level as u8) <= LOG_LEVEL.load(Ordering::Relaxed)
}

#[inline]
pub fn level_str(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "ERROR",
        LogLevel::Warn => "WARN",
        LogLevel::Info => "INFO",
        LogLevel::Debug => "DEBUG",
        LogLevel::Trace => "TRACE",
    }
}

#[macro_export]
macro_rules! log {
    ($level:expr, $component:expr, $($arg:tt)*) => {
        if $crate::log::log_enabled($level) {
            let (secs, frac) = $crate::timer::read_time_parts();
            $crate::println!(
                "[{}.{:0width$}] [{}] [{}] {}",
                secs,
                frac,
                $crate::log::level_str($level),
                $component,
                format_args!($($arg)*),
                width = $crate::timer::TIMEBASE_DECIMALS
            );
        }
    };
}

#[macro_export]
macro_rules! log_error {
    ($component:expr, $($arg:tt)*) => {
        $crate::log!($crate::log::LogLevel::Error, $component, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_warn {
    ($component:expr, $($arg:tt)*) => {
        $crate::log!($crate::log::LogLevel::Warn, $component, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_info {
    ($component:expr, $($arg:tt)*) => {
        $crate::log!($crate::log::LogLevel::Info, $component, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_debug {
    ($component:expr, $($arg:tt)*) => {
        $crate::log!($crate::log::LogLevel::Debug, $component, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_trace {
    ($component:expr, $($arg:tt)*) => {
        $crate::log!($crate::log::LogLevel::Trace, $component, $($arg)*)
    };
}
