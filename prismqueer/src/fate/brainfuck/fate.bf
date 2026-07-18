FATE in Brainfuck
weight injection design

Input 22 bytes: features 0 to 15 then model index then 5 biases
Tape cells 0 to 15 hold features
Tape cell 16 holds model index
Tape cells 17 to 21 hold the five bias scores
Tape cell 28 holds argmax result

PHASE 1 read 22 bytes
dp starts at 0 and ends at 21

,>,>,>,>,>,>,>,>,>,>,>,>,>,>,>,>,>,>,>,>,>,

PHASE 4 add feature 0 to Cartographer score
dp is 21 go left 21 to cell 0

<<<<<<<<<<<<<<<<<<<<<

add cell 0 into cell 19 offset is 19

[>>>>>>>>>>>>>>>>>>>+<<<<<<<<<<<<<<<<<<<-]

dp is 0 go right 28 to cell 28

>>>>>>>>>>>>>>>>>>>>>>>>>>>>

clear cell 28

[-]

go left 11 to cell 17

<<<<<<<<<<<

PHASE 5 argmax over cells 17 to 21
check each cell if nonzero store its index in cell 28

check cell 17 Abyss index 0

[>>>>>>>>>>>[-]<<<<<<<<<<<[-]]

move right to cell 18

>

check cell 18 Introject index 1

[>>>>>>>>>>[-]+<<<<<<<<<<[-]]

move right to cell 19

>

check cell 19 Cartographer index 2

[>>>>>>>>>[-]++<<<<<<<<<[-]]

move right to cell 20

>

check cell 20 Explorer index 3

[>>>>>>>>[-]+++<<<<<<<<[-]]

move right to cell 21

>

check cell 21 Fate index 4

[>>>>>>>[-]++++<<<<<<<[-]]

PHASE 6 output result
go right 7 from cell 21 to cell 28

>>>>>>>
.
