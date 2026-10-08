// SPDX-License-Identifier: GPL-3.0-only
// Pixel-level check of ORIGINAL synthetic geometry, not a game screenshot.
import Foundation
import CoreGraphics
import ImageIO

guard CommandLine.arguments.count == 2,
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: CommandLine.arguments[1]) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
    fatalError("Missing synthetic PNG")
}
let width = image.width, height = image.height
var pixels = [UInt8](repeating: 0, count: width * height * 4)
let counts: (Int, Int, Int) = pixels.withUnsafeMutableBytes { data in
    guard let context = CGContext(data: data.baseAddress, width: width, height: height,
                                 bitsPerComponent: 8, bytesPerRow: width * 4,
                                 space: CGColorSpaceCreateDeviceRGB(),
                                 bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue) else {
        fatalError("Cannot decode synthetic pixels")
    }
    context.draw(image, in: CGRect(x: 0, y: 0, width: CGFloat(width), height: CGFloat(height)))
    let bytes = data.bindMemory(to: UInt8.self)
    var red = 0, cyan = 0, lit = 0
    for i in stride(from: 0, to: bytes.count, by: 4) {
        let r = Int(bytes[i]), g = Int(bytes[i + 1]), b = Int(bytes[i + 2])
        if r > 150 && g < 100 && b < 100 { red += 1 }
        if r < 100 && g > 140 && b > 150 { cyan += 1 }
        if max(r, max(g, b)) > 100 { lit += 1 }
    }
    return (red, cyan, lit)
}
let total = width * height
print("Synthetic screenshot \(width)x\(height): red=\(counts.0) cyan=\(counts.1) lit=\(counts.2)")
// Rotation-independent; a blank frame or the blue/white importer cannot pass.
guard counts.0 > total / 50, counts.1 > total / 50, counts.2 > total / 10 else {
    fatalError("Blank or wrong synthetic scene")
}
print("PASS: both expected synthetic GPU triangles are visible in screenshot pixels")
