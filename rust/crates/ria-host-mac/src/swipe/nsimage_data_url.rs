// Ported from ardia packages/ardiacore/ios/NSImage+DataURL.swift at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSDeviceRGBColorSpace, NSGraphicsContext, NSImage,
};
use objc2_foundation::{
    NSData, NSDataBase64DecodingOptions, NSDataBase64EncodingOptions, NSDictionary, NSPoint,
    NSRect, NSSize, NSString,
};

/// Swift's `Data.pngDataURL`.
pub fn png_data_url(data: &NSData) -> String {
    let base64 = data.base64EncodedStringWithOptions(NSDataBase64EncodingOptions(0));
    format!("data:image/png;base64,{base64}")
}

/// Swift's `NSImage(dataURL:)`.
pub fn image_from_data_url(data_url: &str) -> Option<Retained<NSImage>> {
    let (_, base64) = data_url.strip_prefix("data:")?.split_once(',')?;
    let data = NSData::initWithBase64EncodedString_options(
        NSData::alloc(),
        &NSString::from_str(base64),
        NSDataBase64DecodingOptions(0),
    )?;
    NSImage::initWithData(NSImage::alloc(), &data)
}

/// Swift's `NSImage.pngDataURL(points:scale:)`: the image drawn into a square bitmap of `points` at `scale`.
pub fn image_png_data_url(image: &NSImage, points: f64, scale: f64) -> Option<String> {
    let pixels = (points * scale) as isize;
    // SAFETY: Null planes make AppKit allocate the bitmap; the other arguments describe 8-bit RGBA.
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            pixels,
            pixels,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )
    }?;
    rep.setSize(NSSize::new(points, points));
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(
        NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep).as_deref(),
    );
    image.drawInRect(NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(points, points),
    ));
    NSGraphicsContext::restoreGraphicsState_class();
    // SAFETY: An empty properties dictionary is valid for PNG.
    let png = unsafe {
        rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }?;
    Some(png_data_url(&png))
}
