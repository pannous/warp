done
# play-mozart: a clickable guide section for play "mozart.mp3"

web/playground/guide.md, chapter Sound and music: the plain sentence about `play "mozart.mp3"` became the section
`### Music files` with a `try ▶` snippet `play "samples/sound/mozart.mp3"`. A guide snippet runs without a program
file, so it names the file by its repository path, which the site serves (build.sh copies samples/ at their paths;
https://warp.pannous.com/samples/sound/mozart.mp3 answers 200 audio/mp3). The sample play_mp3 keeps `play "mozart.mp3"`.

direct to main ccdc05e

Tests: test_guide 8/8, test_expert_guide:: 3/3, test_guide_sections:: 1/1, test_guide_heading_links:: 3/3.

Try: https://warp.pannous.com/#guide-sound-and-music → Music files → try ▶
