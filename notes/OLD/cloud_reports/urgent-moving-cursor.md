done
# moving-cursor: moving the cursor away hides the completion

web/playground/completion.js: the completion list closed only when the cursor left the line. Now any cursor move
that comes without a change of the text (arrow keys, a click, Home/End) closes it; typing or Backspace changes the
text in the same CodeMirror operation and keeps updating the list as before. Up/Down while the list is open still
choose in it (they don't move the cursor).

direct to main 39b84c8ae

Tests: none run (express mode); `node --check`.

Try: https://warp.pannous.com/ → type `pri` (the list opens), press ← or click elsewhere: the list closes.
