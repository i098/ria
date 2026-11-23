// Records one window's frames with times through ScreenCaptureKit (works for covered and hidden windows).
//   framegrab <windowID> <out dir> [--fps N] [--max SECONDS]
// stdin: "mark" records the trigger time; "stop" ends the recording and writes <out dir>/frames.json.
// Frame times are milliseconds on the host clock (CLOCK_UPTIME_RAW); a frame is delivered when the window changed.
import AppKit
import ScreenCaptureKit
import CoreMedia

let argv = CommandLine.arguments
guard argv.count >= 3, let windowID = UInt32(argv[1]) else {
  FileHandle.standardError.write("usage: framegrab <windowID> <out dir> [--fps N] [--max SECONDS]\n".data(using: .utf8)!)
  exit(2)
}
let outDir = URL(fileURLWithPath: argv[2])
var fps = 60
var maxSeconds = 30.0
var i = 3
while i + 1 < argv.count {
  if argv[i] == "--fps" { fps = Int(argv[i + 1]) ?? fps } else if argv[i] == "--max" { maxSeconds = Double(argv[i + 1]) ?? maxSeconds }
  i += 2
}
try? FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)

func nowMs() -> Double { Double(clock_gettime_nsec_np(CLOCK_UPTIME_RAW)) / 1e6 }

struct Frame { var ms: Double; var image: CGImage }

final class Grab: NSObject, SCStreamOutput {
  var frames: [Frame] = []
  var marks: [Double] = []
  let ci = CIContext()
  func stream(_ stream: SCStream, didOutputSampleBuffer buffer: CMSampleBuffer, of type: SCStreamOutputType) {
    guard type == .screen, buffer.isValid,
          let attachments = CMSampleBufferGetSampleAttachmentsArray(buffer, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
          let raw = attachments.first?[.status] as? Int, SCFrameStatus(rawValue: raw) == .complete,
          let pixels = CMSampleBufferGetImageBuffer(buffer) else { return }
    let ms = CMTimeGetSeconds(buffer.presentationTimeStamp) * 1000
    let image = CIImage(cvPixelBuffer: pixels)
    guard let cg = ci.createCGImage(image, from: image.extent) else { return }
    frames.append(Frame(ms: ms, image: cg))
  }
}

func downscaled(_ image: CGImage, _ w: Int, _ h: Int) -> [UInt8] {
  var data = [UInt8](repeating: 0, count: w * h * 4)
  let space = CGColorSpaceCreateDeviceRGB()
  data.withUnsafeMutableBytes { p in
    let ctx = CGContext(data: p.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4, space: space,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.interpolationQuality = .medium
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
  }
  return data
}

func meanDiff(_ a: [UInt8], _ b: [UInt8]) -> Double {
  var sum = 0
  for k in 0..<a.count where k % 4 != 3 { sum += abs(Int(a[k]) - Int(b[k])) }
  return Double(sum) / Double(a.count / 4 * 3)
}

let grab = Grab()
var stream: SCStream?

func finish() async {
  try? await stream?.stopCapture()
  let frames = grab.frames
  guard let first = frames.first, let last = frames.last else {
    print("no frames")
    exit(1)
  }
  let w = Int(first.image.width) / 4, h = Int(first.image.height) / 4
  let small = frames.map { downscaled($0.image, w, h) }
  var rows: [[String: Any]] = []
  for (k, f) in frames.enumerated() {
    let name = String(format: "frame-%04d.png", k)
    let rep = NSBitmapImageRep(cgImage: f.image)
    try? rep.representation(using: .png, properties: [:])?.write(to: outDir.appendingPathComponent(name))
    rows.append(["file": name, "ms": f.ms, "diffFirst": meanDiff(small[k], small[0]), "diffLast": meanDiff(small[k], small[small.count - 1])])
  }
  let report: [String: Any] = ["clock": "CLOCK_UPTIME_RAW ms", "fpsRequested": fps, "width": first.image.width, "height": first.image.height,
                               "marks": grab.marks, "frames": rows,
                               "lastSignature": downscaled(last.image, 16, 10).enumerated().filter { $0.offset % 4 != 3 }.map { Int($0.element) }]
  let json = try! JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
  try! json.write(to: outDir.appendingPathComponent("frames.json"))
  print("wrote \(frames.count) frames")
  exit(0)
}

Task {
  do {
    let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)
    guard let window = content.windows.first(where: { $0.windowID == windowID }) else {
      print("no window \(windowID)")
      exit(1)
    }
    let filter = SCContentFilter(desktopIndependentWindow: window)
    let config = SCStreamConfiguration()
    let scale = Int(filter.pointPixelScale)
    config.width = Int(window.frame.width) * scale
    config.height = Int(window.frame.height) * scale
    config.minimumFrameInterval = CMTime(value: 1, timescale: CMTimeScale(fps))
    config.showsCursor = false
    config.queueDepth = 8
    stream = SCStream(filter: filter, configuration: config, delegate: nil)
    try stream!.addStreamOutput(grab, type: .screen, sampleHandlerQueue: DispatchQueue(label: "frames"))
    try await stream!.startCapture()
    print("ready \(nowMs())")
    fflush(stdout)
    DispatchQueue.global().asyncAfter(deadline: .now() + maxSeconds) { Task { await finish() } }
    DispatchQueue.global().async {
      while let line = readLine() {
        if line == "mark" { grab.marks.append(nowMs()); print("marked") ; fflush(stdout) }
        if line == "stop" { Task { await finish() }; return }
      }
    }
  } catch {
    print("error \(error)")
    exit(1)
  }
}
RunLoop.main.run()
