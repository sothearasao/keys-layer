// swift-tools-version: 5.9
import PackageDescription

let package = Package(
  name: "KeysLayerMenu",
  platforms: [.macOS(.v13)],
  products: [
    .executable(name: "KeysLayerMenu", targets: ["KeysLayerMenu"]),
  ],
  targets: [
    .executableTarget(
      name: "KeysLayerMenu",
      path: "Sources"
    ),
  ]
)
