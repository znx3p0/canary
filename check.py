import os

# supported targets are wasm, windows and unix

def check(target, features):
    try:
        os.system(f"cargo check --target={target} --features={features}")
    except Exception as e:
        print(e)
        exit(1)

for args in [
    ["wasm32-unknown-unknown", "static_ser"],
    ["x86_64-pc-windows-gnu", "static_ser"],
    ["x86_64-unknown-linux-gnu", "static_ser"],
    ["x86_64-apple-darwin", "static_ser"],

    ["wasm32-unknown-unknown", ""],
    ["x86_64-pc-windows-gnu", ""],
    ["x86_64-unknown-linux-gnu", ""],
    ["x86_64-apple-darwin", ""],
]:
    print(f'checking -target={args[0]} --features={args[1]}')
    check(args[0], args[1])