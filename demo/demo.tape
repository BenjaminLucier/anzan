# The README's animated demo. record.sh fills in the @...@ placeholders
# and runs this with a fresh demo history.

Output "@OUT@/demo.gif"
Set Shell "bash"
Set FontFamily "Iosevka"
Set FontSize 24
Set Width 1600
Set Height 1000
Set Padding 30
Set Theme "@VHS_THEME@"
Set TypingSpeed 110ms

Hide
# Seed 2 asks 92 + 81, 3 × 82, 7 × 78, 72 − 9, 26 + 2, 720 ÷ 9.
Type "PS1=''; export ANZAN_SEED=2; clear"
Enter
Type "anzan"
Enter
Sleep 2s
Show

# Look over the modes, then start a session in the first.
Sleep 1.5s
Down
Sleep 0.7s
Down
Sleep 0.7s
Up
Sleep 0.5s
Up
Sleep 0.8s
Enter
Sleep 1s

Type "173"
Sleep 0.5s
Type "246"
Sleep 1.5s
# A slip on 7 × 78, put right.
Type "536"
Sleep 0.6s
Backspace 2
Sleep 0.3s
Type "46"
Sleep 0.5s
Type "64"
Sleep 0.3s
Backspace
Type "3"
Sleep 0.5s
Type "28"
Sleep 1.2s
Tab
Sleep 1.5s

# Back to the menu, and on to the statistics.
Escape
Sleep 1s
Type "d"
Sleep 3s
Type "2"
Sleep 0.4s
Down
Sleep 0.4s
Down
Sleep 3s
Type "4"
Sleep 3s
Hide
Escape
Escape
