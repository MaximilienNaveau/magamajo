# fdroid
This repository hosts an [F-Droid](https://f-droid.org/) repo for my apps. This allows you to install and update apps very easily.

### Apps

<!-- This table is auto-generated. Do not edit -->
| Icon | Name | Description | Version |
| --- | --- | --- | --- |
| <a href="https://github.com/MaximilienNaveau/trampoline"><img src="fdroid/repo/icons/com.magamajo.trampoline.3002000.png" alt="trampoline icon" width="36px" height="36px"></a> | [**trampoline**](https://github.com/MaximilienNaveau/trampoline) | "Un jeu pour faire rebondir les mots" par Michel Cheenne. Implementation par ... | 3.2.0 (3002000) |
<!-- end apps table -->

### How to use
1. At first, you should [install the F-Droid app](https://f-droid.org/), it's an alternative app store for Android.
2. Now you can copy the following [link](https://raw.githubusercontent.com/MaximilienNaveau/magamajo/master/fdroid/repo?fingerprint=D04F9C306C0CA32CC89C5D4EA871916330BE95C91FB08F2AD101A11CBFAAC31A), then add this repository to your F-Droid client:

    ```
    https://raw.githubusercontent.com/MaximilienNaveau/magamajo/master/fdroid/repo?fingerprint=D04F9C306C0CA32CC89C5D4EA871916330BE95C91FB08F2AD101A11CBFAAC31A
    ```

    Alternatively, you can also scan this QR code: TBD

    <!-- <p align="center">
      <img src=".github/qrcode.png?raw=true" alt="F-Droid repo QR code"/>
    </p> -->

3. Open the link in F-Droid. It will ask you to add the repository. Everything should already be filled in correctly, so just press "OK".
4. You can now install my apps, e.g. start by searching for "Trampoline" in the F-Droid client.

Please note that some apps published here might contain [Anti-Features](https://f-droid.org/en/docs/Anti-Features/). If you can't find an app by searching for it, you can go to settings and enable "Include anti-feature apps".

### For developers
If you are a developer and want to publish your own apps right from GitHub Actions as an F-Droid repo, you can fork/copy this repo and see  [the documentation](setup.md) for more information on how to set it up.

#### Development Environment

This project uses Nix flakes for reproducible development environments. The Rust toolchain is provided via [rust-overlay](https://github.com/oxalica/rust-overlay), which offers several benefits:

1. **Always up-to-date**: Gets Rust toolchains directly from the official Rust distribution, ensuring you have the latest stable releases immediately (nixpkgs can lag behind by weeks)
2. **Version flexibility**: Easy to pin to specific Rust versions or switch between stable/beta/nightly
3. **Component management**: Cleanly add rust-analyzer, rust-src, clippy as extensions
4. **Consistency**: Uses the same binaries as rustup, avoiding subtle differences

To set up the development environment:
```bash
direnv allow  # If you have direnv installed
# or
nix develop   # To manually enter the dev shell
```

### [License](LICENSE)
The license is for the files in this repository, *except* those in the `fdroid` directory. These files *might* be licensed differently; you can use an F-Droid client to get the details for each app.

### Acknowledgement

This repository is based on this [fdroid](https://github.com/xarantolus/fdroid) repository from [xarantolus](https://github.com/xarantolus).
Please checkout his work if this repository style interest you I used his work to setup my automation.
