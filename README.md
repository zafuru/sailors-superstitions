# Sailors' superstitions
## Usage
```
cargo run -- input_file.csv > output_file.csv
```
## Summary
- Given the time constraints, a couple of things could be improved, like making a macro for case-insensitive matching and refactoring some branches into functions
- The app crashes when the csv is not properly formatted and has extra spaces
- There's a test to handle duplicates and another to handle disputes/resolutions/chargebacks
- 