![repo logo](docs/repo-logo.png)
![repo title](docs/repo-title.png)

---

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=alert_status&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Bugs](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=bugs&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Code Smells](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=code_smells&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=coverage&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Duplicated Lines (%)](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=duplicated_lines_density&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Lines of Code](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=ncloc&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Reliability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=reliability_rating&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Reliability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=reliability_rating&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Technical Debt](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=sqale_index&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Maintainability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=sqale_rating&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_fixdecoder&metric=vulnerabilities&token=693074ba90b11562241b1e602d8dc9ec0ef7bff5)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_fixdecoder)
![Repo Visitors](https://visitor-badge.laobi.icu/badge?page_id=stephenlclarke.fixdecoder)

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
❯ bin/fixdecoder-2.0.3-develop.darwin-arm64 --help
fixdecoder v2.0.3-develop (branch:develop, commit:01dca64)
  git clone git@github.com:stephenlclarke/fixdecoder.git
Usage: fixdecoder [[--fix=44] | [--xml=FIX44.xml]] [--message[=MSG] [--verbose] [--column] [--header] [--trailer]]
       fixdecoder [[--fix=44] | [--xml=FIX44.xml]] [--tag[=TAG] [--verbose] [--column]]
       fixdecoder [[--fix=44] | [--xml=FIX44.xml]] [--component=[NAME] [--verbose]]
       fixdecoder [[--fix=44] | [--xml=FIX44.xml]] [--info]
       fixdecoder [--validate] [--colour=yes|no] [--secret] [file1.log file2.log ...]
       fixdecoder [--version]

Flags:
  -colour
      Force coloured output (yes|no). Default: auto-detect based on stdout
  -column
      Display enums in columns
  -component
      Component to display (omit to list all components)
  -fix string
      FIX version to use (40,41,42,43,44,50,50SP1,50SP2,T11) (default "44")
  -header
      Include Header block
  -info
      Show XML schema summary (fields, components, messages, version counts)
  -message
      Message name or MsgType (omit to list all messages)
  -secret
      Obfuscate sensitive FIX tag values
  -tag
      Tag number to display details for (omit to list all tags)
  -trailer
      Include Trailer block
  -validate
      Validate FIX messages during decoding
  -verbose
      Show full message structure with enums
  -version
      Print version information and exit
  -xml string
      Path to alternative FIX XML file

❯ ./bin/fixdecoder/v2.0.3-develop/fixdecoder --help
fixdecoder v2.0.3-develop (branch:develop, commit:f3c0f91)

  git clone git@github.com:stephenlclarke/fixdecoder.git

Usage: fixdecoder [[--fix=44] | [--xml FIX44.xml]] [--message[=MSG] [--verbose] [--column] [--header] [--trailer]]
       fixdecoder [[--fix=44] | [--xml FIX44.xml]] [--tag[=TAG] [--verbose] [--column]]
       fixdecoder [[--fix=44] | [--xml FIX44.xml]] [--component=[NAME] [--verbose]]
       fixdecoder [[--fix=44] | [--xml FIX44.xml]] [--info]
       fixdecoder [--validate] [--colour=yes|no] [file1.log file2.log ...]

Flags:
  --colour
        Force coloured output (yes|no). Default: auto-detect based on stdout
  --column
        Display enums in columns
  --component
        Component to display (omit to list all components)
  --fix string
        FIX version to use (40,41,42,43,44,50,50SP1,50SP2,T11) (default "44")
  --header
        Include Header block
  --info
        Show XML schema summary (fields, components, messages, version counts)
  --message
        Message name or MsgType (omit to list all messages)
  --secret
        Obfuscate sensitive FIX tag values
  --tag
        Tag number to display details for (omit to list all tags)
  --trailer
        Include Trailer block
  --validate
        Validate FIX messages during decoding
  --verbose
        Show full message structure with enums
  --xml string
        Path to alternative FIX XML file
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
