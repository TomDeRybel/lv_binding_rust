use crate::display::DisplayError;
use crate::Widget;
use core::convert::{TryFrom, TryInto};
#[cfg(feature = "nightly")]
use core::error::Error;
use core::fmt;
use core::ptr::NonNull;
#[cfg(feature = "embedded_graphics")]
use embedded_graphics::pixelcolor::{Rgb565, Rgb888};

pub type LvResult<T> = Result<T, LvError>;

/// Generic LVGL error. All other errors can be coerced into it.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum LvError {
    InvalidReference,
    Uninitialized,
    LvOOMemory,
    AlreadyInUse,
}

impl fmt::Display for LvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LvError::InvalidReference => "Accessed invalid reference or ptr",
                LvError::Uninitialized => "LVGL uninitialized",
                LvError::LvOOMemory => "LVGL out of memory",
                LvError::AlreadyInUse => "Resource already in use",
            }
        )
    }
}

#[cfg(feature = "nightly")]
impl Error for LvError {}

impl From<DisplayError> for LvError {
    fn from(err: DisplayError) -> Self {
        use LvError::*;
        match err {
            DisplayError::NotAvailable => Uninitialized,
            DisplayError::FailedToRegister => InvalidReference,
            DisplayError::NotRegistered => Uninitialized,
        }
    }
}

impl From<LvError> for DisplayError {
    fn from(err: LvError) -> Self {
        use DisplayError::*;
        match err {
            LvError::InvalidReference => FailedToRegister,
            LvError::Uninitialized => NotAvailable,
            LvError::LvOOMemory => FailedToRegister,
            LvError::AlreadyInUse => FailedToRegister,
        }
    }
}

/// An LVGL color. Equivalent to `lv_color_t`.
///
/// This uses the `newtype` design pattern. Combined with #[repr(transparent)],
/// a thin wrapper around the C type is obtained that can be passed across the,
/// and mutated into one-another. Both the struct and the inner field have the
/// same ABI.
/// <https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html>
/// <https://doc.rust-lang.org/nomicon/other-reprs.html#reprtransparent>
#[repr(transparent)]
#[derive(Copy, Clone, Default)]
pub struct Color {
    pub(crate) raw: lvgl_sys::lv_color_t,
}

impl Color {
    /// Creates a `Color` from red, green, and blue values.
    pub fn from_rgb((r, g, b): (u8, u8, u8)) -> Self {
        let raw = unsafe { lvgl_sys::_LV_COLOR_MAKE(r, g, b) };
        Self { raw }
    }
    /// Creates a `Color` from a native `lv_color_t` instance.
    pub fn from_raw(raw: lvgl_sys::lv_color_t) -> Self {
        Self { raw }
    }
    /// Returns the value of the red channel.
    pub fn r(&self) -> u8 {
        unsafe { lvgl_sys::_LV_COLOR_GET_R(self.raw) as u8 }
    }
    /// Returns the value of the green channel.
    pub fn g(&self) -> u8 {
        unsafe { lvgl_sys::_LV_COLOR_GET_G(self.raw) as u8 }
    }
    /// Returns the value of the blue channel.
    pub fn b(&self) -> u8 {
        unsafe { lvgl_sys::_LV_COLOR_GET_B(self.raw) as u8 }
    }
}

#[cfg(feature = "embedded_graphics")]
impl From<Color> for Rgb888 {
    fn from(color: Color) -> Self {
        unsafe {
            Rgb888::new(
                lvgl_sys::_LV_COLOR_GET_R(color.raw) as u8,
                lvgl_sys::_LV_COLOR_GET_G(color.raw) as u8,
                lvgl_sys::_LV_COLOR_GET_B(color.raw) as u8,
            )
        }
    }
}

#[cfg(feature = "embedded_graphics")]
impl From<Color> for Rgb565 {
    fn from(color: Color) -> Self {
        unsafe {
            Rgb565::new(
                lvgl_sys::_LV_COLOR_GET_R(color.raw) as u8,
                lvgl_sys::_LV_COLOR_GET_G(color.raw) as u8,
                lvgl_sys::_LV_COLOR_GET_B(color.raw) as u8,
            )
        }
    }
}

impl From<Color> for lvgl_sys::lv_color_t {
    fn from(val: Color) -> Self {
        val.raw
    }
}

/// Events are triggered in LVGL when something happens which might be interesting to
/// the user, e.g. if an object:
///  - is clicked
///  - is dragged
///  - its value has changed, etc.
///
/// All objects (such as Buttons/Labels/Sliders etc.) receive these generic events
/// regardless of their type.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Event<T> {
    /// The object has been pressed
    Pressed,

    /// The object is being pressed (sent continuously while pressing)
    Pressing,

    /// The input device is still being pressed but is no longer on the object
    PressLost,

    /// Released before `long_press_time` config time. Not called if dragged.
    ShortClicked,

    /// Called on release if not dragged (regardless to long press)
    Clicked,

    /// Pressing for `long_press_time` config time. Not called if dragged.
    LongPressed,

    /// Called after `long_press_time` config in every `long_press_rep_time` ms. Not
    /// called if dragged.
    LongPressedRepeat,

    /// Called in every case when the object has been released even if it was dragged. Not called
    /// if slid from the object while pressing and released outside of the object. In this
    /// case, `Event<_>::PressLost` is sent.
    Released,

    /// Called when an underlying value is changed e.g. position of a `Slider`.
    ValueChanged,

    ///
    DrawMain,

    ///
    DrawMainBegin,

    ///
    DrawMainEnd,

    ///
    DrawPartBegin,

    ///
    DrawPartEnd,

    ///
    DrawPost,

    ///
    DrawPostBegin,

    ///
    DrawPostEnd,

    /// Called on focus
    Focused,

    /// Pointer-like input devices events (E.g. mouse or touchpad)
    Pointer(PointerEvent),

    /// Special event for the object type
    Special(T),
}

impl<S> TryFrom<lvgl_sys::lv_event_code_t> for Event<S> {
    type Error = ();

    fn try_from(value: lvgl_sys::lv_event_code_t) -> Result<Self, Self::Error> {
        const LV_EVENT_PRESSED: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED;
        const LV_EVENT_PRESSING: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_PRESSING;
        const LV_EVENT_PRESS_LOST: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_PRESS_LOST;
        const LV_EVENT_SHORT_CLICKED: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_SHORT_CLICKED;
        const LV_EVENT_CLICKED: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED;
        const LV_EVENT_LONG_PRESSED: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED;
        const LV_EVENT_LONG_PRESSED_REPEAT: u32 =
            lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED_REPEAT;
        const LV_EVENT_RELEASED: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_RELEASED;
        const LV_EVENT_VALUE_CHANGED: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED;
        const LV_EVENT_DRAW_MAIN: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_MAIN;
        const LV_EVENT_DRAW_MAIN_BEGIN: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_MAIN_BEGIN;
        const LV_EVENT_DRAW_MAIN_END: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_MAIN_END;
        const LV_EVENT_DRAW_PART_BEGIN: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_PART_BEGIN;
        const LV_EVENT_DRAW_PART_END: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_PART_END;
        const LV_EVENT_DRAW_POST: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_POST;
        const LV_EVENT_DRAW_POST_BEGIN: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_POST_BEGIN;
        const LV_EVENT_DRAW_POST_END: u32 = lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_POST_END;

        match value {
            LV_EVENT_PRESSED => Ok(Event::Pressed),
            LV_EVENT_PRESSING => Ok(Event::Pressing),
            LV_EVENT_PRESS_LOST => Ok(Event::PressLost),
            LV_EVENT_SHORT_CLICKED => Ok(Event::ShortClicked),
            LV_EVENT_CLICKED => Ok(Event::Clicked),
            LV_EVENT_LONG_PRESSED => Ok(Event::LongPressed),
            LV_EVENT_LONG_PRESSED_REPEAT => Ok(Event::LongPressedRepeat),
            LV_EVENT_RELEASED => Ok(Event::Released),
            LV_EVENT_VALUE_CHANGED => Ok(Event::ValueChanged),
            LV_EVENT_DRAW_MAIN => Ok(Event::DrawMain),
            LV_EVENT_DRAW_MAIN_BEGIN => Ok(Event::DrawMainBegin),
            LV_EVENT_DRAW_MAIN_END => Ok(Event::DrawMainEnd),
            LV_EVENT_DRAW_PART_BEGIN => Ok(Event::DrawPartBegin),
            LV_EVENT_DRAW_PART_END => Ok(Event::DrawPartEnd),
            LV_EVENT_DRAW_POST => Ok(Event::DrawPost),
            LV_EVENT_DRAW_POST_BEGIN => Ok(Event::DrawPostBegin),
            LV_EVENT_DRAW_POST_END => Ok(Event::DrawPostEnd),
            _ => Err(()),
        }
    }
}

impl<S> From<Event<S>> for lvgl_sys::lv_event_code_t {
    fn from(event: Event<S>) -> Self {
        let native_event = match event {
            Event::Pressed => lvgl_sys::lv_event_code_t_LV_EVENT_PRESSED,
            Event::Pressing => lvgl_sys::lv_event_code_t_LV_EVENT_PRESSING,
            Event::PressLost => lvgl_sys::lv_event_code_t_LV_EVENT_PRESS_LOST,
            Event::ShortClicked => lvgl_sys::lv_event_code_t_LV_EVENT_SHORT_CLICKED,
            Event::Clicked => lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
            Event::LongPressed => lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED,
            Event::LongPressedRepeat => lvgl_sys::lv_event_code_t_LV_EVENT_LONG_PRESSED_REPEAT,
            Event::Released => lvgl_sys::lv_event_code_t_LV_EVENT_RELEASED,
            Event::ValueChanged => lvgl_sys::lv_event_code_t_LV_EVENT_VALUE_CHANGED,
            Event::DrawMain => lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_MAIN,
            Event::DrawMainBegin => lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_MAIN_BEGIN,
            Event::DrawMainEnd => lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_MAIN_END,
            Event::DrawPartBegin => lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_PART_BEGIN,
            Event::DrawPartEnd => lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_PART_END,
            Event::DrawPost => lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_POST,
            Event::DrawPostBegin => lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_POST_BEGIN,
            Event::DrawPostEnd => lvgl_sys::lv_event_code_t_LV_EVENT_DRAW_POST_END,
            // TODO: handle all types...
            _ => lvgl_sys::lv_event_code_t_LV_EVENT_CLICKED,
        };
        native_event as lvgl_sys::lv_event_code_t
    }
}

/// Events sent only by pointer-like input devices (e.g. mouse or touchpad)
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum PointerEvent {
    DragBegin,
    DragEnd,
    DragThrowBegin,
}

pub(crate) unsafe extern "C" fn event_callback<'a, T, F>(event: *mut lvgl_sys::lv_event_t)
where
    T: Widget<'a> + Sized,
    F: FnMut(T, Event<<T as Widget<'a>>::SpecialEvent>),
{
    let code = (*event).code;
    let obj = (*event).target;
    // convert the lv_event_code_t to lvgl-rs Event type
    if let Ok(code) = code.try_into() {
        if let Some(obj_ptr) = NonNull::new(obj) {
            let object = T::from_raw(obj_ptr).unwrap();
            // get the pointer from the Rust callback closure FnMut provided by users
            let user_closure = &mut *((*obj).user_data as *mut F);
            // call user callback closure
            user_closure(object, code);
        }
    }
}

/// Possible LVGL alignments for widgets.
pub enum Align {
    Center,
    TopLeft,
    TopMid,
    TopRight,
    BottomLeft,
    BottomMid,
    BottomRight,
    LeftMid,
    RightMid,
    OutTopLeft,
    OutTopMid,
    OutTopRight,
    OutBottomLeft,
    OutBottomMid,
    OutBottomRight,
    OutLeftTop,
    OutLeftMid,
    OutLeftBottom,
    OutRightTop,
    OutRightMid,
    OutRightBottom,
}

impl From<Align> for u8 {
    fn from(value: Align) -> u8 {
        let native = match value {
            Align::Center => lvgl_sys::LV_ALIGN_CENTER,
            Align::TopLeft => lvgl_sys::LV_ALIGN_TOP_LEFT,
            Align::TopMid => lvgl_sys::LV_ALIGN_TOP_MID,
            Align::TopRight => lvgl_sys::LV_ALIGN_TOP_RIGHT,
            Align::BottomLeft => lvgl_sys::LV_ALIGN_BOTTOM_LEFT,
            Align::BottomMid => lvgl_sys::LV_ALIGN_BOTTOM_MID,
            Align::BottomRight => lvgl_sys::LV_ALIGN_BOTTOM_RIGHT,
            Align::LeftMid => lvgl_sys::LV_ALIGN_LEFT_MID,
            Align::RightMid => lvgl_sys::LV_ALIGN_RIGHT_MID,
            Align::OutTopLeft => lvgl_sys::LV_ALIGN_OUT_TOP_LEFT,
            Align::OutTopMid => lvgl_sys::LV_ALIGN_OUT_TOP_MID,
            Align::OutTopRight => lvgl_sys::LV_ALIGN_OUT_TOP_RIGHT,
            Align::OutBottomLeft => lvgl_sys::LV_ALIGN_OUT_BOTTOM_LEFT,
            Align::OutBottomMid => lvgl_sys::LV_ALIGN_OUT_BOTTOM_MID,
            Align::OutBottomRight => lvgl_sys::LV_ALIGN_OUT_BOTTOM_RIGHT,
            Align::OutLeftTop => lvgl_sys::LV_ALIGN_OUT_LEFT_TOP,
            Align::OutLeftMid => lvgl_sys::LV_ALIGN_OUT_LEFT_MID,
            Align::OutLeftBottom => lvgl_sys::LV_ALIGN_OUT_LEFT_BOTTOM,
            Align::OutRightTop => lvgl_sys::LV_ALIGN_OUT_RIGHT_TOP,
            Align::OutRightMid => lvgl_sys::LV_ALIGN_OUT_RIGHT_MID,
            Align::OutRightBottom => lvgl_sys::LV_ALIGN_OUT_RIGHT_BOTTOM,
        };
        native as u8
    }
}

pub enum TextAlign {
    Auto,
    Center,
    Left,
    Right,
}

impl From<TextAlign> for u8 {
    fn from(value: TextAlign) -> Self {
        let native = match value {
            TextAlign::Auto => lvgl_sys::LV_TEXT_ALIGN_AUTO,
            TextAlign::Center => lvgl_sys::LV_TEXT_ALIGN_CENTER,
            TextAlign::Left => lvgl_sys::LV_TEXT_ALIGN_LEFT,
            TextAlign::Right => lvgl_sys::LV_TEXT_ALIGN_RIGHT,
        };
        native as u8
    }
}

/// Boolean for determining whether animations are enabled.
pub enum AnimationState {
    ON,
    OFF,
}

impl From<AnimationState> for lvgl_sys::lv_anim_enable_t {
    fn from(anim: AnimationState) -> Self {
        match anim {
            AnimationState::ON => lvgl_sys::lv_anim_enable_t_LV_ANIM_ON,
            AnimationState::OFF => lvgl_sys::lv_anim_enable_t_LV_ANIM_OFF,
        }
    }
}

#[repr(u32)]
pub enum LabelLongMode {
    Clip = lvgl_sys::LV_LABEL_LONG_CLIP,
    Dot = lvgl_sys::LV_LABEL_LONG_DOT,
    Scroll = lvgl_sys::LV_LABEL_LONG_SCROLL,
    ScrollCircular = lvgl_sys::LV_LABEL_LONG_SCROLL_CIRCULAR,
    Wrap = lvgl_sys::LV_LABEL_LONG_WRAP,
}

impl From<LabelLongMode> for u8 {
    fn from(value: LabelLongMode) -> Self {
        unsafe { (value as u32).try_into().unwrap_unchecked() }
    }
}

/// Possible LVGL object flags.
///
/// This enum contains all LV_OBJ_FLAGS defined in "lvgl-sys/bindings.rs".
///
/// The flags are conceived as bit fields of a 32-bit word, and can thus be
/// logically combined via boolean operations.
///
/// For more detailed information on the flags, see:
/// <https://docs.lvgl.io/master/details/common-widget-features/flags.html>
///
/// TODO: not all LVGL V9 flags are available in the old LVGL V8 used here.
///       Those flags are left commented-out.
///
/// TODO: perhaps use the bitflags! crate instead? See lvgl/lv_core/style.rs
///       for an example.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ObjFlag {
    /// Make the widget hidden (as if it weren’t there at all).
    Hidden,

    /// Make the widget clickable by input devices.
    Clickable,

    /// Add the focused state to the widget when clicked.
    ClickFocusable,

    /// Toggle the checked state when the widget is clicked.
    Checkable,

    /// Make the widget scrollable.
    Scrollable,

    /// Allow elastic scrolling with slower movement.
    ScrollElastic,

    /// Enable momentum scrolling (continue scrolling when “thrown”).
    ScrollMomentum,

    /// Allow scrolling only one snappable child.
    ScrollOne,

    /// Propagate horizontal scrolling to the parent.
    ScrollChainHor,

    /// Propagate vertical scrolling to the parent.
    ScrollChainVer,

    /// Shorthand for (SCROLL_CHAIN_HOR | SCROLL_CHAIN_VER).
    ScrollChain,

    /// Automatically scroll to make the widget visible when focused.
    ScrollOnFocus,

    /// Allow scrolling the focused widget with arrow keys.
    ScrollWithArrow,

    /// Allow the widget to be snapped if the parent has scroll snapping enabled.
    Snappable,

    /// Keep the widget in the pressed state even if the pointer moves outside it.
    PressLock,

    /// Propagate events to the parent.
    EventBubble,

    /// Propagate events to children.
    //EventTrickle,

    /// Propagate state changes to children.
    //StateTrickle,

    /// Propagate gestures to the parent.
    GestureBubble,

    /// Enable more accurate hit (click) testing (e.g., account for rounded corners).
    AdvHitTest,

    /// Exclude the widget from layout positioning.
    IgnoreLayout,

    /// Do not scroll with the parent and ignore layout.
    Floating,

    /// Enable sending LV_EVENT_DRAW_TASK_ADDED events.
    //SendDrawTaskEvents,

    /// Allow children to overflow outside the widget's bounds.
    OverflowVisible,

    /// Start a new flex track on this item.
    //FlexInNewTrack,

    /// Custom flag, free to use by layouts.
    Layout1,

    /// Custom flag, free to use by layouts.
    Layout2,

    /// Custom flag, free to use by widgets.
    Widget1,

    /// Custom flag, free to use by widgets.
    Widget2,

    /// Custom flag, free to use by the user.
    User1,

    /// Custom flag, free to use by the user.
    User2,

    /// Custom flag, free to use by the user.
    User3,

    /// Custom flag, free to use by the user.
    User4,
}

impl From<ObjFlag> for lvgl_sys::lv_obj_flag_t {
    fn from(obj_flag: ObjFlag) -> Self {
        let native_obj_flag = match obj_flag {
            ObjFlag::Hidden => lvgl_sys::LV_OBJ_FLAG_HIDDEN,
            ObjFlag::Clickable => lvgl_sys::LV_OBJ_FLAG_CLICKABLE,
            ObjFlag::ClickFocusable => lvgl_sys::LV_OBJ_FLAG_CLICK_FOCUSABLE,
            ObjFlag::Checkable => lvgl_sys::LV_OBJ_FLAG_CHECKABLE,
            ObjFlag::Scrollable => lvgl_sys::LV_OBJ_FLAG_SCROLLABLE,
            ObjFlag::ScrollElastic => lvgl_sys::LV_OBJ_FLAG_SCROLL_ELASTIC,
            ObjFlag::ScrollMomentum => lvgl_sys::LV_OBJ_FLAG_SCROLL_MOMENTUM,
            ObjFlag::ScrollOne => lvgl_sys::LV_OBJ_FLAG_SCROLL_ONE,
            ObjFlag::ScrollChainHor => lvgl_sys::LV_OBJ_FLAG_SCROLL_CHAIN_HOR,
            ObjFlag::ScrollChainVer => lvgl_sys::LV_OBJ_FLAG_SCROLL_CHAIN_VER,
            ObjFlag::ScrollChain => lvgl_sys::LV_OBJ_FLAG_SCROLL_CHAIN,
            ObjFlag::ScrollOnFocus => lvgl_sys::LV_OBJ_FLAG_SCROLL_ON_FOCUS,
            ObjFlag::ScrollWithArrow => lvgl_sys::LV_OBJ_FLAG_SCROLL_WITH_ARROW,
            ObjFlag::Snappable => lvgl_sys::LV_OBJ_FLAG_SNAPPABLE,
            ObjFlag::PressLock => lvgl_sys::LV_OBJ_FLAG_PRESS_LOCK,
            ObjFlag::EventBubble => lvgl_sys::LV_OBJ_FLAG_EVENT_BUBBLE,
            //ObjFlag::EventTrickle => lvgl_sys::LV_OBJ_FLAG_EVENT_TRICKLE,
            //ObjFlag::StateTrickle => lvgl_sys::LV_OBJ_FLAG_STATE_TRICKLE,
            ObjFlag::GestureBubble => lvgl_sys::LV_OBJ_FLAG_GESTURE_BUBBLE,
            ObjFlag::AdvHitTest => lvgl_sys::LV_OBJ_FLAG_ADV_HITTEST,
            ObjFlag::IgnoreLayout => lvgl_sys::LV_OBJ_FLAG_IGNORE_LAYOUT,
            ObjFlag::Floating => lvgl_sys::LV_OBJ_FLAG_FLOATING,
            //ObjFlag::SendDrawTaskEvents => lvgl_sys::LV_OBJ_FLAG_SEND_DRAW_TASK_EVENTS,
            ObjFlag::OverflowVisible => lvgl_sys::LV_OBJ_FLAG_OVERFLOW_VISIBLE,
            //ObjFlag::FlexInNewTrack => lvgl_sys::LV_OBJ_FLAG_FLEX_IN_NEW_TRACK,
            ObjFlag::Layout1 => lvgl_sys::LV_OBJ_FLAG_LAYOUT_1,
            ObjFlag::Layout2 => lvgl_sys::LV_OBJ_FLAG_LAYOUT_2,
            ObjFlag::Widget1 => lvgl_sys::LV_OBJ_FLAG_WIDGET_1,
            ObjFlag::Widget2 => lvgl_sys::LV_OBJ_FLAG_WIDGET_2,
            ObjFlag::User1 => lvgl_sys::LV_OBJ_FLAG_USER_1,
            ObjFlag::User2 => lvgl_sys::LV_OBJ_FLAG_USER_2,
            ObjFlag::User3 => lvgl_sys::LV_OBJ_FLAG_USER_3,
            ObjFlag::User4 => lvgl_sys::LV_OBJ_FLAG_USER_4,
        };
        native_obj_flag as lvgl_sys::lv_obj_flag_t
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn color_properties_accessible() {
        let color = Color::from_rgb((206, 51, 255));

        if lvgl_sys::LV_COLOR_DEPTH == 32 {
            assert_eq!(color.r(), 206);
            assert_eq!(color.g(), 51);
            assert_eq!(color.b(), 255);
        } else if lvgl_sys::LV_COLOR_DEPTH == 16 {
            assert_eq!(color.r(), 25);
            assert_eq!(color.g(), 12);
            assert_eq!(color.b(), 31);
        }
    }
}
