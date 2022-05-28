
import os

# supported targets are wasm, windows and unix

def check(args):
    try:
        os.system(f"cargo check {args}")
    except Exception as e:
        print(e)
        exit(1)

for target in  [
    "--target=wasm32-unknown-unknown",
    "--target=x86_64-pc-windows-msvc",
    "--target=x86_64-unknown-linux-gnu",
    "--target=x86_64-apple-darwin",
    "",
]:
    for feature in [
        ""
    ]:
        print(target, feature)
        check(f'{target} {feature}')
