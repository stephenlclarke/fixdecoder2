#!/bin/bash
# fixdecoder — FIX protocol decoder tools
# Copyright (C) 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# In accordance with section 13 of the AGPL, if you modify this program,
# your modified version must prominently offer all users interacting with it
# remotely through a computer network an opportunity to receive the source
# code of your version.


# Prefer local sonar-scanner if available
if command -v sonar-scanner >/dev/null 2>&1; then
  echo "Using local sonar-scanner"
  sonar-scanner -Dsonar.token="${SONAR_TOKEN}" "$@"
else
  echo "Local sonar-scanner not found, falling back to Docker image"
  docker run --rm \
    --platform=linux/amd64 \
    -e SONAR_TOKEN="${SONAR_TOKEN}" \
    -v "$(pwd):/usr/src" \
    sonarsource/sonar-scanner-cli "$@"
fi
