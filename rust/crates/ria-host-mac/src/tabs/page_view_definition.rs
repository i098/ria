// Ported from ardia packages/ardiacore/ios/PageViewDefinition.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! `PageViewDefinition`: the props and the functions that a page view offers the app, and what each one does to the
//! page. Expo generates the bridge in Ardia; here `set_prop` and `call` take the name and the JSON values that the
//! bridge carries. Function arguments are positional, as in the Expo call, and a missing optional argument is `null`.

use objc2::rc::Retained;
use objc2_app_kit::NSColor;
use serde_json::{Map, Value, json};

use super::page_tab::Reply;
use super::page_tab_host::PageTabHost;

#[derive(Debug, Eq, PartialEq)]
pub enum CallError {
    Unknown(String),
    /// The argument at `index` is missing or has the wrong type.
    BadArgument {
        name: String,
        index: usize,
        expected: &'static str,
    },
}

/// A promise that rejects: `promise.reject(code, message)`.
#[derive(Debug, Eq, PartialEq)]
pub struct Rejection {
    pub code: &'static str,
    pub message: String,
}

/// A call that answers nothing: run it, then `null`.
macro_rules! void {
    ($call:expr) => {{
        $call;
        Value::Null
    }};
}

pub type Settled = Reply<Result<Value, Rejection>>;

struct Args<'a> {
    name: &'a str,
    list: &'a [Value],
}

impl Args<'_> {
    fn bad(&self, index: usize, expected: &'static str) -> CallError {
        CallError::BadArgument {
            name: self.name.to_owned(),
            index,
            expected,
        }
    }

    fn at(&self, index: usize) -> Option<&Value> {
        self.list.get(index).filter(|v| !v.is_null())
    }

    fn opt<'v, T>(
        &'v self,
        index: usize,
        expected: &'static str,
        take: fn(&'v Value) -> Option<T>,
    ) -> Result<Option<T>, CallError> {
        self.at(index)
            .map(|v| take(v).ok_or_else(|| self.bad(index, expected)))
            .transpose()
    }

    fn opt_str(&self, index: usize) -> Result<Option<&str>, CallError> {
        self.opt(index, "string", Value::as_str)
    }

    fn str(&self, index: usize) -> Result<&str, CallError> {
        self.opt_str(index)?
            .ok_or_else(|| self.bad(index, "string"))
    }

    fn opt_bool(&self, index: usize) -> Result<Option<bool>, CallError> {
        self.opt(index, "boolean", Value::as_bool)
    }

    fn bool(&self, index: usize) -> Result<bool, CallError> {
        self.opt_bool(index)?
            .ok_or_else(|| self.bad(index, "boolean"))
    }

    fn opt_f64(&self, index: usize) -> Result<Option<f64>, CallError> {
        self.opt(index, "number", Value::as_f64)
    }

    fn f64(&self, index: usize) -> Result<f64, CallError> {
        self.opt_f64(index)?
            .ok_or_else(|| self.bad(index, "number"))
    }

    fn opt_int(&self, index: usize) -> Result<Option<i64>, CallError> {
        self.opt(index, "integer", Value::as_i64)
    }

    fn int(&self, index: usize) -> Result<i64, CallError> {
        self.opt_int(index)?
            .ok_or_else(|| self.bad(index, "integer"))
    }

    fn strings(&self, index: usize) -> Result<Vec<String>, CallError> {
        let list = self
            .at(index)
            .and_then(Value::as_array)
            .ok_or_else(|| self.bad(index, "array of strings"))?;
        list.iter()
            .map(|v| v.as_str().map(str::to_owned))
            .collect::<Option<_>>()
            .ok_or_else(|| self.bad(index, "array of strings"))
    }

    fn object(&self, index: usize) -> Result<&Map<String, Value>, CallError> {
        self.at(index)
            .and_then(Value::as_object)
            .ok_or_else(|| self.bad(index, "object"))
    }
}

/// The Expo `NSColor` convertible: a CSS hex string (`#rgb`, `#rrggbb`, `#rrggbbaa`) or a number `0xAARRGGBB`, which
/// is what React Native's `processColor` gives [INFERENCE: read from the Expo and React Native conventions, not run].
fn color(value: &Value) -> Option<Retained<NSColor>> {
    let (r, g, b, a) = match value {
        Value::String(text) => {
            let hex = text.strip_prefix('#')?;
            let digits: Vec<u32> = hex.chars().map(|c| c.to_digit(16)).collect::<Option<_>>()?;
            match digits[..] {
                [r, g, b] => (r * 17, g * 17, b * 17, 255),
                [r1, r0, g1, g0, b1, b0] => (r1 * 16 + r0, g1 * 16 + g0, b1 * 16 + b0, 255),
                [r1, r0, g1, g0, b1, b0, a1, a0] => {
                    (r1 * 16 + r0, g1 * 16 + g0, b1 * 16 + b0, a1 * 16 + a0)
                }
                _ => return None,
            }
        }
        Value::Number(n) => {
            let argb = u32::try_from(n.as_u64()?).ok()?;
            (
                (argb >> 16) & 255,
                (argb >> 8) & 255,
                argb & 255,
                argb >> 24,
            )
        }
        _ => return None,
    };
    let unit = |v: u32| f64::from(v) / 255.0;
    Some(NSColor::colorWithSRGBRed_green_blue_alpha(
        unit(r),
        unit(g),
        unit(b),
        unit(a),
    ))
}

/// A `Prop`: `null` is the default of the prop. `mediaCaptureSourceId` and `frozen` are functions, not props.
pub fn set_prop(host: &PageTabHost, name: &str, value: &Value) -> Result<(), CallError> {
    let args = Args {
        name,
        list: std::slice::from_ref(value),
    };
    let page = &host.page;
    let string = || args.opt_str(0).map(|v| v.map(str::to_owned));
    let flag = |default: bool| args.opt_bool(0).map(|v| v.unwrap_or(default));
    let paint = || match args.at(0) {
        None => Ok(None),
        Some(v) => color(v).map(Some).ok_or_else(|| args.bad(0, "color")),
    };
    match name {
        "pageEvents" => host.set_page_events(args.at(0).map(|_| args.strings(0)).transpose()?),
        "url" => page.set_initial_url(string()?),
        "profile" => page.set_profile(string()?.unwrap_or_default()),
        "adoptId" => page.set_adopt_id(string()?),
        "tabId" => page.set_transfer_key(string()?),
        "standalone" => page.set_standalone(flag(false)?),
        "temporary" => page.set_temporary(flag(false)?),
        "temporarySource" => page.set_temporary_source(string()?),
        "extensionHost" => page.set_extension_host(string()?),
        "visible" => page.set_visible(flag(true)?),
        "selected" => page.set_selected(flag(true)?),
        "warm" => page.set_warm(flag(false)?),
        "neighbour" => page.set_neighbour(flag(false)?),
        "pageBackgroundColor" => page.set_page_background_color(paint()?),
        "pageColor" => page.set_page_color(string()?),
        "contentsBackgroundColor" => page.set_contents_background_color(paint()?),
        "shownUrl" => page.set_shown_url(string()?),
        "autoPictureInPicture" => page.set_auto_picture_in_picture(flag(false)?),
        _ => return Err(CallError::Unknown(name.to_owned())),
    }
    Ok(())
}

/// `OnViewDidUpdateProps`.
pub fn view_did_update_props(host: &PageTabHost) {
    host.props_did_update();
}

/// One `AsyncFunction`. `reply` settles once, on the main thread, with the value the promise resolves to.
pub fn call(
    host: &PageTabHost,
    name: &str,
    list: &[Value],
    reply: Settled,
) -> Result<(), CallError> {
    let a = Args { name, list };
    let page = &host.page;
    let value: Value = match name {
        "loadUrl" => void!(page.load_url(a.str(0)?, a.opt_bool(1)?.unwrap_or(false))),
        "goBack" => void!(page.go_back()),
        "goForward" => void!(page.go_forward()),
        "goToOffset" => void!(page.go_to_history_offset(a.int(0)?)),
        "reload" => void!(page.reload()),
        "forceReload" => void!(page.reload_ignoring_cache()),
        "stopLoading" => void!(page.stop_loading()),
        "focus" => void!(page.focus_page()),
        "focusIfUnfocused" => void!(page.focus_page_if_unfocused()),
        "setMuted" => void!(page.set_muted(a.bool(0)?)),
        "zoomStep" => void!(page.zoom_step(a.int(0)?)),
        "find" => void!(page.find(a.str(0)?, a.bool(1)?, a.bool(2)?)),
        "stopFinding" => void!(page.stop_finding(a.bool(0)?)),
        "print" => void!(page.print()),
        "runPageCommand" => void!(page.run_page_command(a.str(0)?)),
        "showDevTools" => void!(page.show_dev_tools(a.opt_str(0)?)),
        "executeJavaScript" => void!(page.execute_javascript(a.str(0)?)),
        "evaluate" => {
            page.evaluate(
                a.str(0)?,
                Box::new(move |v| reply(Ok(v.map_or(Value::Null, Value::String)))),
            );
            return Ok(());
        }
        "navigationEntries" => {
            page.navigation_entries(Box::new(move |v| reply(Ok(Value::Array(v)))));
            return Ok(());
        }
        "capturePicture" => {
            let scale = a.f64(0)?;
            page.capture_picture(
                scale,
                Box::new(move |v| reply(Ok(v.unwrap_or(Value::Null)))),
            );
            return Ok(());
        }
        "downloadFavicon" => {
            page.download_favicon(
                a.str(0)?,
                Box::new(move |v| reply(Ok(v.unwrap_or(Value::Null)))),
            );
            return Ok(());
        }
        "downloadImage" => {
            page.download_image(
                a.str(0)?,
                a.int(1)?,
                Box::new(move |v| reply(Ok(v.unwrap_or(Value::Null)))),
            );
            return Ok(());
        }
        "mediaCommand" => void!(page.media_command(a.str(0)?, a.opt_f64(1)?.unwrap_or(0.0))),
        "requestPictureInPicture" => {
            let (click_at, control) = (a.opt_f64(0)?.unwrap_or(0.0), a.opt_int(1)?.unwrap_or(0));
            page.request_picture_in_picture(
                click_at,
                control,
                Box::new(move |v| reply(Ok(json!(v)))),
            );
            return Ok(());
        }
        "takeControlPress" => {
            json!(page.take_control_press(a.opt_f64(0)?.unwrap_or(0.0), a.opt_int(1)?.unwrap_or(0)))
        }
        "exitPictureInPicture" => void!(page.exit_picture_in_picture()),
        "armElementPick" => json!(page.arm_element_pick()),
        "takeElementPick" => json!(page.take_element_pick(a.f64(0)?)),
        "getSecurityInfo" => {
            page.security_info(Box::new(move |v| reply(Ok(v))));
            return Ok(());
        }
        "pageCookies" => {
            let urls = a.strings(0)?;
            page.page_cookies(
                &urls,
                Box::new(move |v| reply(Ok(Value::Array(v.unwrap_or_default())))),
            );
            return Ok(());
        }
        "siteData" => {
            page.site_data(
                a.object(0)?,
                Box::new(move |v| reply(Ok(v.unwrap_or(Value::Null)))),
            );
            return Ok(());
        }
        "openBlockedPopup" => {
            void!(page.open_blocked_popup(a.str(0)?, a.opt_bool(1)?.unwrap_or(false)))
        }
        "clearSiteData" => {
            page.clear_site_data(Box::new(move |r| {
                reply(r.map_err(|message| Rejection {
                    code: "ERR_ENGINE",
                    message,
                }));
            }));
            return Ok(());
        }
        "resolvePasswordPrompt" => {
            void!(page.resolve_password_prompt(a.str(0)?, a.opt_str(1)?, a.opt_str(2)?))
        }
        "resolveAutofillPrompt" => void!(page.resolve_autofill_prompt(a.int(0)?, a.str(1)?)),
        "answerAutofillPrompt" => json!(page.answer_autofill_prompt(a.int(0)?, a.str(1)?)),
        "executeExtensionAction" => json!(page.execute_extension_action(a.str(0)?)),
        "resolveDisplayMedia" => {
            void!(page.resolve_display_media(a.str(0)?, a.opt_str(1)?, a.bool(2)?))
        }
        "mediaCaptureSourceId" => json!(page.media_capture_source_id()),
        "notificationAction" => void!(page.notification_action(a.str(0)?, a.str(1)?)),
        "resolveUnresponsive" => void!(page.resolve_unresponsive(a.bool(0)?)),
        "discard" => json!(page.discard(a.opt_bool(0)?.unwrap_or(false))),
        "sleepIfHidden" => json!(page.sleep_if_hidden(a.f64(0)?)),
        "setFrozen" => void!(page.set_frozen(a.bool(0)?)),
        _ => return Err(CallError::Unknown(name.to_owned())),
    };
    reply(Ok(value));
    Ok(())
}

#[cfg(test)]
mod tests {
    use objc2_app_kit::NSColorSpace;

    use super::*;

    fn rgba(value: Value) -> Option<[f64; 4]> {
        let color = color(&value)?.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
        Some(
            [
                color.redComponent(),
                color.greenComponent(),
                color.blueComponent(),
                color.alphaComponent(),
            ]
            .map(|v| (v * 255.0).round()),
        )
    }

    #[test]
    fn colors_accept_hex_text_and_argb_numbers() {
        assert_eq!(rgba(json!("#f80")), Some([255.0, 136.0, 0.0, 255.0]));
        assert_eq!(rgba(json!("#102030")), Some([16.0, 32.0, 48.0, 255.0]));
        assert_eq!(rgba(json!("#10203040")), Some([16.0, 32.0, 48.0, 64.0]));
        assert_eq!(
            rgba(json!(0x80_10_20_30_u32)),
            Some([16.0, 32.0, 48.0, 128.0])
        );
        assert_eq!(rgba(json!("red")), None);
        assert_eq!(rgba(json!("#12")), None);
    }
}
