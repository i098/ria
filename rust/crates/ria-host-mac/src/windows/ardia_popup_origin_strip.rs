// Ported from ardia packages/ardiacore/ios/ArdiaPopupOriginStrip.mm at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, define_class, msg_send};
use objc2_app_kit::{
    NSColor, NSCompositingOperation, NSFont, NSFontWeightMedium, NSImage,
    NSImageSymbolConfiguration, NSImageView, NSLineBreakMode, NSRectFill, NSRectFillUsingOperation,
    NSTextField, NSView,
};
use objc2_foundation::{
    NSIntersectionRect, NSPoint, NSRect, NSSize, NSString, NSURLComponents, ns_string,
};

fn host_and_port(parts: &NSURLComponents) -> Option<String> {
    let host = parts.encodedHost()?.lowercaseString();
    if host.length() == 0 {
        return None;
    }
    Some(match parts.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    })
}

pub fn popup_origin_text(url: &str) -> String {
    if url.is_empty() {
        return "about:blank".to_owned();
    }
    let Some((parts, scheme)) = NSURLComponents::componentsWithString(&NSString::from_str(url))
        .and_then(|parts| Some((parts.clone(), parts.scheme()?.lowercaseString())))
    else {
        return url.find(':').map_or(url, |at| &url[..=at]).to_owned();
    };
    let scheme = scheme.to_string();
    match scheme.as_str() {
        "blob" | "filesystem" => {
            if let Some((inner_scheme, host)) = parts
                .percentEncodedPath()
                .and_then(|path| NSURLComponents::componentsWithString(&path))
                .and_then(|inner| Some((inner.scheme()?.lowercaseString(), host_and_port(&inner)?)))
            {
                format!("{scheme}:{inner_scheme}://{host}")
            } else {
                format!("{scheme}:")
            }
        }
        _ => match host_and_port(&parts) {
            Some(host) if scheme == "https" => host,
            Some(host) => format!("{scheme}://{host}"),
            None if matches!(scheme.as_str(), "about" | "file") => {
                let path = parts
                    .path()
                    .map(|path| path.to_string())
                    .unwrap_or_default();
                let slashes = if scheme == "file" { "//" } else { "" };
                format!("{scheme}:{slashes}{path}")
            }
            None => format!("{scheme}:"),
        },
    }
}

pub struct OriginStripIvars {
    glyph: Retained<NSImageView>,
    origin: Retained<NSTextField>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "RiaPopupOriginStrip"]
    #[ivars = OriginStripIvars]
    pub struct PopupOriginStrip;

    impl PopupOriginStrip {
        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool {
            true
        }

        #[unsafe(method(mouseDownCanMoveWindow))]
        fn mouse_down_can_move_window(&self) -> bool {
            true
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            // SAFETY: This calls NSView's implementation with its declared NSSize argument.
            unsafe { let _: () = msg_send![super(self), setFrameSize: size]; }
            self.arrange(size);
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, dirty: NSRect) {
            let bounds = self.bounds();
            NSColor::windowBackgroundColor().setFill();
            NSRectFill(NSIntersectionRect(dirty, bounds));
            NSColor::separatorColor().setFill();
            let density = self.window().map_or(1., |window| window.backingScaleFactor().max(1.));
            NSRectFillUsingOperation(rect(0., 0., bounds.size.width, 1. / density), NSCompositingOperation::SourceOver);
        }
    }
);

impl PopupOriginStrip {
    pub fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let glyph = NSImageView::initWithFrame(mtm.alloc(), rect(0., 0., 0., 0.));
        // SAFETY: NSFontWeightMedium is an immutable AppKit constant.
        let symbol = NSImageSymbolConfiguration::configurationWithPointSize_weight(11., unsafe {
            NSFontWeightMedium
        });
        glyph.setSymbolConfiguration(Some(&symbol));
        let origin = NSTextField::labelWithString(ns_string!(""), mtm);
        origin.setFont(Some(&NSFont::systemFontOfSize(12.)));
        origin.setTextColor(Some(&NSColor::labelColor()));
        origin.setLineBreakMode(NSLineBreakMode::ByTruncatingHead);
        let this = mtm
            .alloc::<Self>()
            .set_ivars(OriginStripIvars { glyph, origin });
        // SAFETY: NSView's designated initializer receives an allocated view with initialized ivars.
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        this.setClipsToBounds(true);
        this.addSubview(&this.ivars().glyph);
        this.addSubview(&this.ivars().origin);
        this.arrange(frame.size);
        this
    }

    fn arrange(&self, size: NSSize) {
        let ivars = self.ivars();
        let text = ivars.origin.intrinsicContentSize().height.ceil();
        ivars
            .glyph
            .setFrame(rect(10., ((size.height - 16.) / 2.).floor(), 16., 16.));
        ivars.origin.setFrame(rect(
            31.,
            ((size.height - text) / 2.).floor(),
            (size.width - 41.).max(0.),
            text,
        ));
    }

    pub fn show_url(&self, url: &str, level: Option<&str>) {
        let ivars = self.ivars();
        ivars
            .origin
            .setStringValue(&NSString::from_str(&popup_origin_text(url)));
        let scheme = NSURLComponents::componentsWithString(&NSString::from_str(url))
            .and_then(|parts| parts.scheme())
            .map(|scheme| scheme.to_string().to_ascii_lowercase());
        let scheme = scheme.as_deref();
        let level = level.unwrap_or(if matches!(scheme, Some("http" | "ws")) {
            "insecure"
        } else {
            "none"
        });
        let (symbol, description) = match level {
            "secure" => (ns_string!("lock.fill"), ns_string!("Connection is secure")),
            "insecure" | "mixed" | "certificateError" => (
                ns_string!("lock.open.trianglebadge.exclamationmark.fill"),
                ns_string!("Connection is not secure"),
            ),
            _ if matches!(scheme, Some("https" | "wss")) => (
                ns_string!("info.circle"),
                ns_string!("Connection security unknown"),
            ),
            _ => (
                ns_string!("info.circle"),
                ns_string!("Local or internal page"),
            ),
        };
        let tint = if level == "certificateError" {
            NSColor::colorWithSRGBRed_green_blue_alpha(1., 95. / 255., 87. / 255., 1.)
        } else {
            NSColor::secondaryLabelColor()
        };
        let image =
            NSImage::imageWithSystemSymbolName_accessibilityDescription(symbol, Some(description));
        ivars.glyph.setImage(image.as_deref());
        ivars.glyph.setContentTintColor(Some(&tint));
    }
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}
