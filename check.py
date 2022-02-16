import os

# supported targets are wasm, windows and unix
os.system("cargo check --target=wasm32-unknown-unknown")
os.system("cargo check --target=x86_64-pc-windows-gnu")
os.system("cargo check --target=x86_64-unknown-linux-gnu")
