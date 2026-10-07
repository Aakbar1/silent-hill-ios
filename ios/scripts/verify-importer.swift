// SPDX-License-Identifier: GPL-3.0-only
// macOS Vision checks actual simulator screenshot pixels, not an app log alone.
import Foundation
import ImageIO
import Vision

guard CommandLine.arguments.count == 2 else { fatalError("Expected importer screenshot path") }
let url = URL(fileURLWithPath: CommandLine.arguments[1])
var observed = ""
var passed = false
// simctl screenshots retain the physical display's portrait pixel orientation.
for orientation: CGImagePropertyOrientation in [.up, .right, .down, .left] {
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = false
    request.recognitionLanguages = ["en-US"]
    try VNImageRequestHandler(url: url, orientation: orientation, options: [:]).perform([request])
    let text = (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }.joined(separator: " ")
    observed += "orientation \(orientation.rawValue): \(text)\n"
    let lower = text.lowercased()
    if lower.contains("import your silent hill") && lower.contains("usa") && lower.contains("disc image") && lower.contains("bin") && lower.contains("choose") {
        passed = true
        break
    }
}
print(observed)
guard passed else { fputs("FAIL: required importer title and choose button were not visible in screenshot.\n", stderr); exit(1) }
print("PASS: importer title and choose button recognized in screenshot pixels.")
