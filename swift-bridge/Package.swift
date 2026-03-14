// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "nudge-bridge",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "nudge-bridge",
            path: "Sources"
        ),
    ]
)
