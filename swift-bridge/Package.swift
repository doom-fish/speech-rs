// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "SpeechBridge",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "SpeechBridge",
            type: .static,
            targets: ["SpeechBridge"])
    ],
    targets: [
        .target(
            name: "SpeechBridge",
            path: "Sources/SpeechBridge",
            publicHeadersPath: "include")
    ]
)
