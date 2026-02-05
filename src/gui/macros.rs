//! Layout macros for GUI widgets
//!
//! These macros provide convenient syntax for creating column and row layouts.

/// Creates a column layout with optional children.
///
/// # Examples
///
/// ```
/// // Empty column
/// let col = column![];
///
/// // Column with children
/// let col = column![text1, text2, text3];
/// ```
#[macro_export]
macro_rules! column {
    () => (
        $crate::gui::widget::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::gui::widget::Column::with_children([$($crate::gui::widget::Element::from($x)),+])
    );
}

/// Creates a row layout with optional children.
///
/// # Examples
///
/// ```
/// // Empty row
/// let row = row![];
///
/// // Row with children
/// let row = row![button1, button2, button3];
/// ```
#[macro_export]
macro_rules! row {
    () => (
        $crate::gui::widget::Row::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::gui::widget::Row::with_children([$($crate::gui::widget::Element::from($x)),+])
    );
}
