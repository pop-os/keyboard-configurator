# Macros

## Files

- `build.py` - Builds with Cargo, generates a `.app` bundle, and build a `.dmg`
- `Info.plist.in` - Used to generate `Info.plist`, processing with Python `.format()`
- `appdmg.json` - Configuration file for [node-appdmg](https://github.com/LinusU/node-appdmg)

## Building

`./build.py --relase` should generate a `.dmg`
