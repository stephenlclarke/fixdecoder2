#!/usr/bin/env bash

delay=10   # default sleep delay

# Parse options
while getopts "d:" opt; do
    case "$opt" in
        d) delay="$OPTARG" ;;
        *) echo "Usage: $0 [-d delay] [file...]" >&2; exit 1 ;;
    esac
done

shift $((OPTIND - 1))

slow_output() {
    while IFS= read -r line; do
        echo "$line"
        sleep "$delay"
    done
}

# If no files provided, read from stdin
if [ $# -eq 0 ]; then
    slow_output
else
    for file in "$@"; do
        if [ "$file" = "-" ]; then
            slow_output
        else
            slow_output < "$file"
        fi
    done
fi