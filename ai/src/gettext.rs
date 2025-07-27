// Same as formatx! but returns the original string if gettext fails.
#[macro_export]
macro_rules! formatget {
    ($fmt:expr $(, $args:expr)* $(,)?) => {{
        let fmt_str: &str = $fmt.as_ref();
        formatx::formatx!(gettext(fmt_str) $(, $args)*).unwrap_or($fmt.to_string())
    }};
}
