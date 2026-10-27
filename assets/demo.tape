# Records assets/demo.gif with https://github.com/charmbracelet/vhs
#
# The tuner listens to the real microphone, so have your ukulele ready:
#   vhs assets/demo.tape
# then pluck each string and turn the peg while the recording runs.

Output assets/demo.gif

Set FontSize 20
Set Width 1100
Set Height 320
Set Padding 24
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms

Type "uketuner"
Sleep 500ms
Enter

# G string: pluck it slightly flat and tighten it until the meter turns green
Sleep 9s
Enter

# C string
Sleep 7s
Type "q"
Sleep 1s
