use crate::ui::panel::PanelMode;

use super::panel::Panel;

pub const DOC: &str = "# Help

## List mode

q      quit

j,k    next/prev item
h,l    up/down level
g      go to top
G      go to bottom

n,p    new post
r      reply to this post
d      delete this post
U      update this post

s      open setting

## Input panel

j,k    next/prev
i,e    input
s      commit edit
ESC    complete input / back to List view without save";

pub(super) fn help_panel() -> Panel {
    Panel::new(vec![], DOC, PanelMode::Info)
}
