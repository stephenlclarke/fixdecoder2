![repo logo](docs/repo-logo.png)
![repo title](docs/repo-title.png)

---

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Bugs](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=bugs)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Code Smells](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=code_smells)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=coverage)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Duplicated Lines (%)](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=duplicated_lines_density)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Lines of Code](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=ncloc)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Reliability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=reliability_rating)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Security Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=security_rating)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Technical Debt](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=sqale_index)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Maintainability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=sqale_rating)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder2&metric=vulnerabilities)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder2)
![Repo Visitors](https://visitor-badge.laobi.icu/badge?page_id=stephenlclarke.fixdecoder2)

---

# Steve's FIX Decoder / logfile prettify utility

This is my attempt to create an "all-singing / all-dancing" utility to pretty-print logfiles containing FIX Protocol messages while simultaneously learning **Rust** (after first building an earlier version in Go) and trying to incorporate SonarQube Code Quality metrics.

I have written utilities like this in past in Java, Python, C, C++ and even in Bash/Awk!! This is my favourite one so far — and now it is fully native Rust.

![repo title](docs/example.png)

---

<p align="center">
  <a href="https://buy.stripe.com/8x23cvaHjaXzdg30Ni77O00">
    <img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-❤️-brightgreen?style=for-the-badge&logo=buymeacoffee&logoColor=white" alt="Buy Me a Coffee">
  </a>
  &nbsp;
  <a href="https://github.com/stephenlclarke/fixdecoder/discussions">
    <img src="https://img.shields.io/badge/Leave%20a%20Comment-💬-blue?style=for-the-badge" alt="Leave a Comment">
  </a>
</p>

<p align="center">
  <sub>☕ If you found this project useful, consider buying me a coffee or dropping a comment — it keeps the caffeine and ideas flowing! 😄</sub>
</p>

---

# How to use it

The utility behaves like the `cat` utility in `Linux`, except as it reads the input (either piped in from `stdin` or from a filename specified on the commandline) it scans each line for `FIX protocol` messages and prints them out highlighted in bold white while the rest of the line will be in a mid grey colour. After the line is output it will be followed by a detailed breakdown of all the `FIX Protocol` tags that were found in the message. The detailed output will use the appropriate `FIX` dictionary for the version of `FIX` specified in `BeginString (tag 8)` tag.

I plan to produce an update shortly that will also look at `DefaultApplVerID (tag 1137)` when `8=FIXT.1.1` is detected in the message.

## Running the utility

```bash
❯ target/debug/fixdecoder --help
fixdecoder v0.1.0 (branch:main, commit:f54194a)

FIX protocol decoder tools

Usage: fixdecoder [OPTIONS] [FILE]...

Arguments:
  [FILE]...  

Options:
      --fix <VER>           FIX version to use [default: 44]
      --xml <FILE>          Path to alternative FIX XML dictionary (repeatable)
      --message [<MSG>]     FIX Message name or MsgType (omit value to list all)
      --component [<NAME>]  FIX Component to display (omit value to list all)
      --tag [<TAG>]         FIX Tag number to display (omit value to list all)
      --column              Display enums in columns
      --header              Include Header block
      --trailer             Include Trailer block
      --verbose             Show full message structure with enums
      --info                Show schema summary
      --secret              Obfuscate sensitive FIX tag values
      --validate            Validate FIX messages during decoding
      --colour [<yes|no>]   Force coloured output
      --delimiter <CHAR>    Display delimiter between FIX fields (default: SOH)
      --version             Print version information and exit
  -h, --help                Print help

Command line option examples:

  fixdecoder [[--fix=44] [--xml=FILE --xml=FILE2 ...]] [--message[=NAME|MSGTYPE] [--verbose] [--column] [--header] [--trailer] [--delimiter=CHAR]]
  fixdecoder [[--fix=44] [--xml=FILE --xml=FILE2 ...]] [--tag[=TAG] [--verbose] [--column] [--delimiter=CHAR]]
  fixdecoder [[--fix=44] [--xml=FILE --xml=FILE2 ...]] [--component[=NAME] [--verbose] [--column] [--delimiter=CHAR]]
  fixdecoder [[--fix=44] [--xml=FILE --xml=FILE2 ...]] [--info]
  fixdecoder [--xml=FILE --xml=FILE2 ...] [--validate] [--colour=yes|no] [--secret] [--delimiter=CHAR] [file1.log file2.log ...]
  fixdecoder [--version]
```

```bash
❯ target/debug/fixdecoder --info
fixdecoder v0.1.0 (branch:main, commit:f54194a)

Available FIX Dictionaries: FIX27,FIX30,FIX40,FIX41,FIX42,FIX43,FIX44,FIX50,FIX50SP1,FIX50SP2,FIXT11

Loaded dictionaries:
  Version     ServicePack   Fields  Components    Messages Source
  FIX27                 0      138           2          27 built-in
  FIX30                 0      138           2          27 built-in
  FIX40                 0      138           2          27 built-in
  FIX41                 0      206           2          28 built-in
  FIX42                 0      405           2          46 built-in
  FIX43                 0      635          12          68 built-in
  FIX44                 0      912         106          93 built-in
  FIX50                 0     1090         123          93 built-in
  FIX50SP1              1     1373         165         105 built-in
  FIX50SP2              2     6028         727         156 built-in
  FIXT11                0       71           4           8 built-in
```

## How to get it

ℹ️ However you download it you will have to make the binary executable on your
computer. **Windows** users will need to rename the download and add a `.exe`
extension to the binary before you can execute it. **Linux** and **MacOS**
users will need to do a `chmod +x` on the file first.

### Download it

Check out the Repo's [Releases Page](https://github.com/stephenlclarke/fixdecoder/releases)
to see what versions are available for the computer you want to run it on.

### Build it

Build it from source. This now requires `bash` version 5+ and a recent `Rust` toolchain (the project is tested with Rust 1.78+).

```bash
❯ bash --version
GNU bash, version 5.3.3(1)-release (aarch64-apple-darwin24.4.0)
Copyright (C) 2025 Free Software Foundation, Inc.
License GPLv3+: GNU GPL version 3 or later <http://gnu.org/licenses/gpl.html>

This is free software; you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.

❯ rustc --version
rustc 1.78.0 (9b00956e5 2024-04-29)
```

Clone the git repo.

```bash
❯ git clone git@github.com:stephenlclarke/fixdecoder.git
Cloning into 'fixdecoder'...
remote: Enumerating objects: 418, done.
remote: Counting objects: 100% (418/418), done.
remote: Compressing objects: 100% (375/375), done.
remote: Total 418 (delta 201), reused 0 (delta 0), pack-reused 0 (from 0)
Receiving objects: 100% (418/418), 1.02 MiB | 2.65 MiB/s, done.
Resolving deltas: 100% (201/201), done.
❯ cd fixdecoder
```

Then build it.

```bash
❯ cargo build --release
   Compiling fixdecoder v2.1.0 (/Users/you/fixdecoder)
    Finished `release` profile [optimized] target(s) in 7.37s
```

Run it (from the optimized build) and check the version details:

```bash
❯ ./target/release/fixdecoder --version
fixdecoder v2.1.0 (branch:develop, commit:c2a60e8)
  git clone git@github.com:stephenlclarke/fixdecoder.git
```

# Third-Party Specifications

This project uses the public FIX Protocol XML specifications from the
[QuickFIX project](https://github.com/quickfix/quickfix/tree/master/spec).
The XML files are downloaded during the build and used to generate Go sources
under `fix/` and to drive message decoding at runtime.

The QuickFIX specifications are licensed under the **BSD 2-Clause License**.
Their copyright notice and license terms are included in this repository’s
[`NOTICE`](./NOTICE) file (and in `licenses/QUICKFIX-BSD-2-Clause.txt`).

---

© 2025 Steve Clarke · Released under the [AGPL-3.0 License](https://www.gnu.org/licenses/agpl-3.0.html)

---
