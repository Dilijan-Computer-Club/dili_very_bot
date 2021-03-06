// Mostly copied from pretty_env_logger https://github.com/seanmonstar/pretty-env-logger

use std::fmt;
use std::sync::atomic::{Ordering, AtomicUsize};
use env_logger::{
    fmt::{Color, Style, StyledValue},
    Builder,
};
use log::Level;

// pub extern crate env_logger;
// extern crate log;

pub fn init() {
    let mut builder = formatted_builder();

    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    }

    builder.try_init().unwrap();
}

/// Returns a `env_logger::Builder` for further customization.
///
/// This method will return a colored and formatted `env_logger::Builder`
/// for further customization. Refer to env_logger::Build crate documentation
/// for further details and usage.
pub fn formatted_builder() -> Builder {
    let mut builder = Builder::new();

    builder.format(|f, record| {
        use std::io::Write;

        let target = record.target();
        let max_width = max_target_width(target);

        let mut style = f.style();
        let level = colored_level(&mut style, record.level());

        let mut style = f.style();
        let target = style.set_bold(true).value(Padded {
            value: target,
            width: max_width,
        });

        writeln!(f, " {} {} > {}", level, target, record.args(),)
    });

    builder
}
struct Padded<T> {
    value: T,
    width: usize,
}

impl<T: fmt::Display> fmt::Display for Padded<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{: <width$}", self.value, width = self.width)
    }
}

static MAX_MODULE_WIDTH: AtomicUsize = AtomicUsize::new(0);

fn max_target_width(target: &str) -> usize {
    let max_width = MAX_MODULE_WIDTH.load(Ordering::Relaxed);
    if max_width < target.len() {
        MAX_MODULE_WIDTH.store(target.len(), Ordering::Relaxed);
        target.len()
    } else {
        max_width
    }
}

fn colored_level<'a>(style: &'a mut Style, level: Level) -> StyledValue<'a, &'static str> {
    match level {
        Level::Trace => style.set_color(Color::Magenta).value("TRACE"),
        Level::Debug => style.set_color(Color::Blue).value("DEBUG"),
        Level::Info => style.set_color(Color::Green).value("INFO "),
        Level::Warn => style.set_color(Color::Yellow).value("WARN "),
        Level::Error => style.set_color(Color::Red).value("ERROR"),
    }
}
