// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2025 Steve Clarke <stephenlclarke@mac.com> - https://xyzzy.tools
//
// Shared layout constants for FIX rendering across prettifier and schema display.

/// Width used when printing tag numbers (right-aligned).
pub const TAG_WIDTH: usize = 4;
/// Base indent applied to top-level prettifier fields.
pub const BASE_INDENT: usize = 2;
/// Indent increment for nested components/groups.
pub const NEST_INDENT: usize = 4;
/// Column offset to align group separators under the first parenthesis of the field name.
pub const NAME_TEXT_OFFSET: usize = TAG_WIDTH + 1;
/// Indent applied to entries inside a repeating group (relative to the group's own indent).
pub const ENTRY_FIELD_INDENT: usize = TAG_WIDTH + 1;
