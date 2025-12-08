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

I have written utilities like this in past in Java, Python, C, C++, [go](https://github.com/stephenlclarke/fixdecoder) and even in Bash/Awk!! This is my favourite one so far — and now it is fully native Rust.

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

The utility behaves like the `cat` utility in `Linux`, except as it reads the input (either piped in from `stdin` or from a filename specified on the commandline) it scans each line for `FIX protocol` messages and prints them out highlighted in bold white while the rest of the line will be in a mid grey colour. After the line is output it will be followed by a detailed breakdown of all the `FIX Protocol` tags that were found in the message. The detailed output will use the appropriate `FIX` dictionary for the version of `FIX` specified in `BeginString (tag 8)` tag. It will also look at `DefaultApplVerID (tag 1137)` when `8=FIXT.1.1` is detected in the message.

## Running the utility

```bash
❯ ./target/release/fixdecoder --help
fixdecoder 0.2.0 (branch:develop, commit:7a2d535) [rust:1.91.1]
FIX protocol utility - Dictionary lookup, file decoder, validator & prettifier

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
      --summary             Track order state across messages and print a summary
  -h, --help                Print help

Command line option examples:
  Query FIX dictionary contents by FIX Message Name or MsgType:
    fixdecoder [[--fix=44] [--xml=FILE --xml=FILE2 ...]] [--message[=NAME|MSGTYPE] [--verbose] [--column] [--header] [--trailer]

    $ fixdecoder --message=NewOrderSingle --verbose --column --header --trailer
    $ fixdecoder --message=D --verbose --column --header --trailer
  
  Query FIX dictionary contents by FIX Tag number:
    fixdecoder [[--fix=44] [--xml=FILE --xml=FILE2 ...]] [--tag[=TAG] [--verbose] [--column]

    $ fixdecoder --tag=44 --verbose --column
    
  Query FIX dictionary contents by FIX Component Name:
    fixdecoder [[--fix=44] [--xml=FILE --xml=FILE2 ...]] [--component[=NAME] [--verbose] [--column]

    $ fixdecoder --component=Instrument --verbose --column

  Show summary information about available FIX dictionaries:
    fixdecoder [[--fix=44] [--xml=FILE --xml=FILE2 ...]] [--info]

    $ fixdecoder --info

  Prettify FIX log files with optional validation and obfuscation if output is piped then colour is disabled by default but can be forced on with --colour=yes:
    fixdecoder [--xml=FILE --xml=FILE2 ...] [--validate] [--colour=yes|no] [--secret] [--summary] [--fix=VER] [--delimiter=CHAR] [file1.log file2.log ...]

    $ fixdecoder --validate --secret --summary logs/fix.log
    $ grep '35=D' logs/fix.log | fixdecoder --colour=yes --delimiter='|' --summary | less
    $ fixdecoder --fix=44 trades.log   (forces FIX44 decoding instead of auto-detect)
    $ tail -f logs/fix.log | fixdecoder --validate
```

```bash
❯ target/debug/fixdecoder --info
fixdecoder 0.2.0 (branch:develop, commit:7a2d535) [rust:1.91.1]
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

## Download it

Check out the Repo's [Releases Page](https://github.com/stephenlclarke/fixdecoder2/releases)
to see what versions are available for the computer you want to run it on.

## Build it

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
❯ make build-release

>> Ensuring Rust toolchain and coverage tools

>> Installing llvm-tools-preview component
info: component 'llvm-tools' for target 'aarch64-apple-darwin' is up to date

>> Ensuring FIX XML specs are present
   Compiling fixdecoder v0.2.0 (/Users/sclarke/github/fixdecoder2)
warning: fixdecoder@0.2.0: Building fixdecoder 0.2.0 (branch:develop, commit:7a2d535) [rust:1.91.1]
    Finished `release` profile [optimized] target(s) in 2.21s
```

Run it (from the optimized build) and check the version details:

```bash
❯ ./target/release/fixdecoder --version
fixdecoder 0.2.0 (branch:develop, commit:7a2d535) [rust:1.91.1]
  git clone git@github.com:stephenlclarke/fixdecoder2.git
```

# Technical Notes on the use of the `--summary` flag

- As messages stream by, the decoder builds one “record” per order (keyed by OrderID/ClOrdID/OrigClOrdID).
- Each message updates that record: standard fields (Side, Symbol, Qty, Price, TIF, OrdType, TradeDate, SettlDate) are taken from the latest message; BN messages also set ExecAckStatus, Spot Price (LastPx), and ExecAmt (38).
- The header row shows the order key, the flow of states observed (OrdStatus/ExecType/ExecAckStatus), and a table of the latest known values: Side/Symbol/Qty/Price/TradeDate/Tenor/TIF/OrdType/ValueDate (tag 64/193). Prices include currency when present.
- The timeline lists every message for the order with columns: time, msg (enum text plus ClOrdID/OrigClOrdID), ExecAckStatus (for BN), ExecType, OrdStatus, cum/leaves, last@price, avgPx, text. Enums show text; unknown codes show in red; missing text shows as “-” in green.
- Tenor is computed from TradeDate to ValueDate skipping weekends; SPOT = T+2, TOM = T+1, TOD = T+0, otherwise FWD. (no holiday calendars).
- If a `--fix` override cannot be found, decoding falls back to the auto-detected dictionary with a warning on stderr and a banner at runtime.

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
