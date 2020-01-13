Brainfuck Self Interpreter: by Clive Gifford 

Version 1: 01 December 2006 (one trip between code and data per op)
Version 2: 16 December 2006 (virtual codes for various combinations)

Credits

A large section of code to load the input to the interpreter is copied 
from the the 423 byte dbfi interpreter as described in the November 2003
paper "A Very Small Interpeter" by Oleg Mazonka and Daniel B Cristofani

Goals

The goal for this interpreter was to be efficient rather than small and
particularly to allow as many copies of itself as possible to be "stacked"
up with something else on top / In other words to achieve a low "eigenratio"
(See http eigenratios dot blogspot dot com for more information)

The main idea of the first version was to only make one round trip between
the emulated code and emulated data for each instruction executed instead
of multiple round trips which is what Daniel and Oleg's version does

The second version does more pre processing of the guest program in order
to map several common sequences to virtual codes thus reducing the memory
footprint and also further reducing the number of round trips between the
emulated code and data

Other info:

The input must consist of valid brainfuck code (to be interpreted) which
must always be followed by an exclamation mark and then any associated data
Input can also include "comments" (if desired) except for exclamation mark

If you are stacking multiple copies of this interpreter then each additional
level also has to appear in the input with a trailing exclamation mark and 
then we finally have the input for the very top level to finish things off

The underlying brainfuck machine determines the possible range of values in
the data cell values and what happens if an attempt is made to go outside the
supported range but this interpreter does not more than 8 bit data itself

Loops in the emulated code can be nested up to the maximum cell value in the
underlying machine and this interpreter requires that at least 17 levels of
nesting is supported

Behaviour on end of input is also inherited from the next level down

>>>>                        leave a little extra space before the program code
+                           start in left hand cell of first program code pair
[
  ->>>                      clear flag and move to the starting position
  ++>+>+++++++              setup differences and read char as per dbfi
  [<++++>>++<-]++>>+>+>     (by Daniel B Cristofani)
  +++++[>++>++++++<<-]
  +>>>,<++
  [
    [>[->>]<[>>]<<-]        see section 3 of dbfi paper
     <[<]<+>>[>]>
    [                       see section 4 of dbfi paper
      <+>-
      [[<+>-]>]             see section 5 of dbfi paper
      <                     see section 6 of dbfi paper
      [
        [[-]<]              scan left and zero the differences
        ++<-                see section 7 of dbfi paper
        [
          <+++++++++>[<->-]>>
        ]
        >>
      ]
    ]
    <<
  ]

 a three way switch to handle possibilities and adjust positioning
 
  >[-]+<<                   set "done" flag and position to value
  [
    --                      2 means last decode was for a valid instruction
    [                       so if we still have something left now it was a 9
      [-]                   originally (meaning input should be ignored)
      >>- <<<+>             so all we do is set the "more input" flag
    ]
    >> 
    [
      -
      <<<<[>+<-]            for valid input move everything right one

 start of processing to find and recode certain combinations of codes   

      +<<+                  set flags to indicate we want to process 
                            the last two instructions in the loop below
      [                     
        ->                  clear flag and move to instruction
        [<+>>>>>>+<<<<<-]   double copy instruction and then
        <[>+<-]>>>>>>       restore original before moving to copy

 map relevant codes to values giving unique pairwise sums and also
 set a flag if we see the end of a loop

        >+<
        [
         -[
           -[
             -[
               -[
                 -[
                   -[
                     -[
                       -[
                          [-]
                          >-<
                        ]>[-<<+++++++>>] <
                      ]
                    ]
                  ]>[-]<
                ]>[-<<+++>>]<
              ]>[-<<+>>]<
            ]>[-]<
          ]>[-<<<<<<<+>>>>>>>]<   set flag if it is end of loop
        ]>[-]<
        <<<<                      goto next instruction flag
      ]

 add values from above together to get unique sum

      >>>[<<+>>-]
      <+<

 setup a switch to figure out what it means

      [
       -[
         -[ 
           -[
             -[
               -[
                 -[
                   -[
                     -[
                       -[
                         -[
                            [-]
                            >-<<<[-]<<+>>  change code 8 (add 1) to 9 (add 2)
                          ]
                        ]
                      ]
                    ]>[-]<
                  ]>[-<<<[-]<<+++++++>>>]< change code 4 (left) to 11 (left 2)
                ]
              ]
            ]>[-]<
          ]>[-<<<[-]<<+++++++>>>]<         change code 3 (right) to 10 (right 2)
        ]
      ]>[-]<

 clear flag set if second to last was end of loop
 and go to similar flag for last instruction

      <<<<<[-]>>
      [
        -
        <<<[>+>>+<<<-]>[<+>-]>>            copy third to last instruction
 
 if it is the start of a loop then we can (in some cases)
 collapse the last three instructions to single virtual code

        [
         -[
           -[
              [-]
              >>
            ]
            >
            [    Now we are ready to check what code is in the loop (must be at least 1)
              <<[<+>>+<-]>[<+>-]+<<
              -
              [
               -[
                 -[
                   -[
                     -[
                       -[
                         -[
                           -[
                             -[
                               -[
                                  -
                                  <+>         fall through & code as 16 (double skip left)
                                ]<+++++++++++++>>[-]>->-<<   code as 15 (double skip right)
                              ]                  
                            ]
                          ]>>[->>>>>]<<
                        ]>>[-<<<++++++++++++>>[-]>>-]<<      code as 14 (zero)
                      ]>>[->>>>>]<<
                    ]>>[-<<<+++++++++++>>[-]>>-]<<           code as 13 (skip left)
                  ]>>[-<<<++++++++++>>[-]>>-]<<              code as 12 (skip right)
                ]                         
              ]>>[->>>>>]<<
            ]
            <
          ]
        ]>[>>]<
        <<
      ]
      >>+>>>
    ] 
    <<
  ]
  >> [->+>] <<              end of input so clear "done" and set data mark and
                            finally position to a zero cell ready for next phase
  
  <                         move to "more input" flag
  
]

<<[<<]>>                    go to the first instruction

                            ******** MAIN INTERPRETER LOOPS STARTS HERE ********

[                           start on current instruction code
                            setup a big switch statement to decode instructions
  [<+>>+<-]                 move/copy instruction code and set "done" flag to
  +<-                       start with '(i less 1) (1) (i) for i = instruction
  [
   -[
     -[
       -[
         -[
           -[
             -[
               -[
                 -[
                   -[
                     -[
                       -[
                         -[
                           -[
                             -[
                                -           can't be anything but 1 so bracketing not needed
                                >->> [>>] >> [>>]< [<-<<-<] <[<<] << [<<]<   double skip left (code 16)
                              ]
                              >[->> [>>] >> [>>]< [>+>>+>] <[<<] << [<<]]<   double skip right (code 15)
                            ]
                            >[->> [>>] >> [>>] <[-]< [<<] << [<<]]<  zero (code 14)
                          ]
                          >[->> [>>] >> [>>]< [<-<] <[<<] << [<<]]<  skip left (code 13)
                        ]
                        >[->> [>>] >> [>>]< [>+>] <[<<] << [<<]]<    skip right  (code 12)
                      ] 
                      >[->> [>>] >> [>>]< <-<<-< <[<<] << [<<]]<  double move left (code 11)
                    ]
                    >[->> [>>] >> [>>] +>>+ [<<] << [<<]]<        double move right (code 10)
                  ]
                  >[->> [>>] >> [>>]< ++ <[<<] << [<<]]<          add 2 (code 9)
                ]
                >[->> [>>] >> [>>]< + <[<<] << [<<]]<       increment
              ]
              >[->> [>>] >> [>>]< , <[<<] << [<<]]<         input
            ]
            >[->> [>>] >> [>>]< - <[<<] << [<<]]<           decrement
          ]
          >[->> [>>] >> [>>]< . <[<<] << [<<]]<             output
        ]
        >[->> [>>] >> [>>] <<-<< [<<] << [<<]]<             move left
      ]
      >[->> [>>] >> [>>] + [<<] << [<<]]<                   move right
    ]
    >
    [-                      left hand bracket
      >> [>>] >> [>>]<      move to data cell
      [>+>>+<<<-]>          make double copy and move to first
      [<+>-]                restore original data cell value
      >>[<<+>>[-]]+         This and the following achieves
      <<[>>-<<-]            x = not x
      >>                    go to flag cell (0 or 1)

      Some tricky stuff here: set up (not flag) also so we can later choose
      appropriate instruction sequence to get back to code area in one pass
      In one case we set flags at the other end (data greater than 0) but
      for the other we just go back without setting any flags (data equals 0)

      [<<+>>>>+<<-]         make two copies of flag
      >>[<<+>>-]
      <<[>>+<<-]+           This and the following achieves
      >>[<<->>-]<<          x = not x

      <<                    so we now have (data) '(flag) (?) (not flag)

      [                     if flag set then
        -<< [<<] << [<<]<   clear and return to code section where we save
        << << ++            a 2 meaning we need (later) to match left bracket
        >>                  stop in zero cell for now
      ]

      >>                    if we executed code above then now at switch flag
                            else it will put us ready to return from data area

      [-<<<<<<[<<]<<[<<]<]  move back to switch flag without setting anything

      >
    ]
    <
  ]
  >
  [-                        right hand bracket
    >> [>>] >> [>>]<        move to data cell
    [>+>>+<<<-]>            make double copy and move to first
    [[<+>-]>>[-]+<<]        restore data from one then zero second and set flag
    >>                      go to flag cell (0 or 1)

    Some tricky stuff here: set up (not flag) also so we can later choose
    appropriate instruction sequence to get back to code area in one pass
    In one case we set flags at the other end (data greater than 0) but
    for the other we just go back without setting any flags (data equals 0)

    [<<+>>>>+<<-]           make two copes of flag
    >>[<<+>>-]
    <<[>>+<<-]+             This and the following achieves
    >>[<<->>-]<<            x = not x
       
    <<                      so we now have (data) '(flag) (?) (not flag)
    
    [                       if flag set then
      -<< [<<] << [<<]<     clear and return to code section where we save
      << << +               a 1 meaning we need (later) to match right bracket
      >>                    stop in zero cell for now
    ]

    >>                      if we executed code above then now at switch flag
                            else it will put us ready to return from data area
                          
    [-<<<<<<[<<]<<[<<]<]    move back to switch flag without setting anything
    
    >
  ]

  >[<+>-]                   restore original instruction code

  *** We are positioned in the cell immediately to the right of the   ***
  *** instruction that has just been "executed" in the switch above   ***
  *** The following code is to handle finding matching brackets       ***
  *** because code above has only set a cell value to 1 or 2 to show  ***
  *** what kind of loop scanning is required (1=scan left 2=right)    ***
  
  << << <<                  position to cell showing if matching required
  [                         if non zero we need to find a matching bracket
    >> +                    set up "done" flag for switch and
    << -                    decrement switch value so now is 0 or 1
    [                       if 1 we are looking for matching right bracket
      - >> - >> +           clear switch value & "done" & set level to 1
      [                     while level is more than 0
        >>>[-<+>>+<]        make double copy of instruction code
        +<-                 set flag and prepare for switch
        [
         -[
            [-]             clear whatever is left of code
            > - <           do nothing except clear flag
          ]
          > [- <<< + >>>] < increment level
        ]
        > [- <<< - >>>]     decrement level

        >[-<+>]<<           restore instruction code

        <<                  go to level
        [>>+<<-]            if level then move right one instruction
        >>
      ]
      << << <<              go back to switch value cell
    ]
    >>                      go to switch done flag and if still set then
    [                       we must be looking for a matching left bracket
      - << +                clear switch value & "done" & set level to 1
      [                     repeat while level is more than 0
        >>>[-<+>>+<]        make double copy of instruction code
        +<-                 set flag and prepare for switch
        [
         -[
            [-]             clear whatever is left of code
            > - <           do nothing except clear flag
          ]
          > [- <<< - >>>] < decrement level
        ]
        > [- <<< + >>>]     increment level

        >[-<+>]<<           restore instruction code

        <<                  go to level
        [<<+>>-]            if level then move left one instruction
        <<
      ]
    ]
  ]

  >> >> >>

  >                         move forward to next instruction
]
