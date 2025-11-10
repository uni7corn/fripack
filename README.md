# Fripack
### Package your Frida script into an executable.

Frida is great. However, it's heavy. With the requirement of root permission, it's hard to allow users to use the script we developed on their own phone, thus it's hard to use Frida to develop plugins.

Fripack package your script into Xposed Module, Zygisk Module, .so to LD_PRELOAD, .dll to inject, etc. It enables you to develop plugins in frida and distribute them with ease.

## Installation
Download the binary from the [latest release](https://github.com/std-microblock/fripack/releases/latest) and do whatever u like.

## Getting Started

### Basic 
The configuration file of Fripack is `fripack.json`. It a JSON5 file like this:

```json
{
    "xposed": {
        "type": "xposed",
        "version": "1.0.0",
        "fridaVersion": "17.5.1",
        "entry": "main.js",
        "xz": true,
        "outputDir": "./fripack",
        "platform": "arm64-v8a",
        "packageName": "com.example.myxposedmodule",
        "keystore": "./.android/debug.keystore",
        "keystorePass": "android",
        "keystoreAlias": "androiddebugkey",
        "name": "My Xposed Module"
    }
}
```

Each object is a target to compile to. You can use `fripack build` to build all targets, or `fripack build xposed`, for example, to build a specific target.

### Universal Configs
- `xz` (default: false): Compress the script with lzma or not.
- `entry` (required): The script to bundle
- `fridaVersion` (required): The version of frida to use. Must be 17.5.1+.
- `outputDir` (default: ./): The directory to put the output.
- `platform`: x86_64, x64, arm64-v8a, etc.
- `version`: Version of your plugin.
- `type`: Type of the target.
- `inherit`: Key of the target to inherit config from.

For example, you can write
```json
{
    "base": {
        "version": "1.0.0",
        "fridaVersion": "17.5.1",
        "entry": "main.js",
        "xz": true,
        "outputDir": "./fripack",
        "platform": "arm64-v8a",
    },
    "xposed": {
        "inherit": "base",
        "type": "xposed",
        "packageName": "com.example.myxposedmodule",
        "keystore": "./.android/debug.keystore",
        "keystorePass": "android",
        "keystoreAlias": "androiddebugkey",
        "name": "My Xposed Module"
    },
    "raw-so": {
        "inherit": "base",
        "type": "android-so",
    }
}
```
to avoid repeated definitions. Only targets with a `type` will be built.

### Types
The type is in what form you'd like to build your frida script to. The currently available forms are:

#### `xposed`
This will build your frida script into a xposed module. [`apktool`](https://apktool.org/) is required to be installed in your machine for this to work.

- `sign` (optional): Should Fripack sign the generated APK. Requires `apksigner` to be installed to work.
  - `keystore` (required if `sign`): The keystore to sign
  - `keystorePass` (required if `sign`): Passphrase of the keystore
  - `keystoreAlias` (required if `sign`): Alias to choose in the keystore

- `packageName` (required): Package name of the Xposed Module
- `name` (required): Name of the xposed module.
.....
