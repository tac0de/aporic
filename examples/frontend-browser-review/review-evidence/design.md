# Design contract

The example has one primary task: open a proposal, inspect its review guidance,
and mark the basic check complete. The opening hierarchy is title, progress,
filter, then proposal cards. The dialog presents the summary and three checks
before the completion action.

The visual system uses a warm neutral background, dark text, green for reviewed
state, and rust for section labels. Serif headings distinguish the proposal
content from controls. Cards collapse from three columns to two and then one;
the filter and dialog remain usable on a narrow viewport.

All actionable elements use native buttons or checkboxes. The dialog has a
label and description, supports Escape, and restores focus when dismissed.
The reviewed count is announced through a status region. Motion is limited to
brief color transitions and disabled under reduced-motion preference.

This is a local design hypothesis and implementation contract. Visual judgment
comes from the recorded browser inspection, not from file existence.
