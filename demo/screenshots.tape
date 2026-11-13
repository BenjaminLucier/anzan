# The README's screenshots, once per theme. record.sh fills in the @...@
# placeholders and runs this with a fresh demo history.

Output "@TMP@/screenshots-@THEME@.gif"
Set Shell "bash"
Set FontFamily "Iosevka"
Set FontSize 24
Set Width 1600
Set Height 1000
Set Padding 30
Set Theme "@VHS_THEME@"

Hide
Type "PS1=''; clear"
Enter
Show

# The menu, then the dashboard: the overview, multiplication with its
# times tables, and the slowest weak spot.
Type "anzan"
Enter
Sleep 2s
Screenshot "@OUT@/@THEME@/menu.png"
Sleep 0.5s
Type "d"
Sleep 2s
Screenshot "@OUT@/@THEME@/dashboard.png"
Sleep 0.5s
Type "2"
Down
Down
Sleep 1s
Screenshot "@OUT@/@THEME@/dashboard-multiplication.png"
Sleep 0.5s
Type "4"
Sleep 1s
Screenshot "@OUT@/@THEME@/dashboard-weak-spot.png"
Sleep 0.5s
Escape
Sleep 0.5s
Escape
Sleep 1s

# A few kinds of question, each from a seed chosen for it. Leaving before
# answering anything records nothing.
Type "clear; ANZAN_SEED=5 anzan --kinds mul --duration 300"
Enter
Sleep 2s
Type "32"
Sleep 0.5s
Screenshot "@TMP@/@THEME@-question-multiplication.png"
Sleep 0.5s
Escape
Sleep 0.5s
Escape
Sleep 1s

Type "clear; ANZAN_SEED=19 anzan --kinds convert --duration 300"
Enter
Sleep 2s
Screenshot "@TMP@/@THEME@-question-fraction.png"
Sleep 0.5s
Escape
Sleep 0.5s
Escape
Sleep 1s

Type "clear; ANZAN_SEED=1 anzan --kinds sqrt --duration 300"
Enter
Sleep 2s
Screenshot "@TMP@/@THEME@-question-square-root.png"
Sleep 0.5s
Escape
Sleep 0.5s
Escape
Sleep 1s

Type "clear; ANZAN_SEED=2 anzan --kinds square --duration 300"
Enter
Sleep 2s
Screenshot "@TMP@/@THEME@-question-square.png"
Sleep 0.5s
Escape
Sleep 0.5s
Escape
Sleep 1s
