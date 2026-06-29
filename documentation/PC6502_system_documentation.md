# 6809PC
The 6502PC is a 6502 ATX format board with 512K RAM, 4K ROM, with a flexible programmable MMU, battery backed up RTC, 6551 UART, and 6 ISAish slots.  This computer will run a version of DOS/65 by Richard A. Leary.  

See this repo (https://github.com/danwerner21/6x0x-DOS65) for ROM image and operating system.

![System](images/6502PC.jpg)


### Jumper Settings
        J1 - IRQ ASSIGNMENT FOR SLOT J2
        J2 - IRQ ASSIGNMENT FOR SLOT J4
        J5 - IRQ ASSIGNMENT FOR SLOT J6
        J7 - IRQ ASSIGNMENT FOR SLOT J8
        J9 - IRQ ASSIGNMENT FOR SLOT J10        
        J13 - IRQ ASSIGNMENT FOR SLOT J14
        
        K1 - ROM BANK SELECT 1&2-ROM IMAGE AT $6000-$6FFF  3&4-ROM IMAGE AT $7000-$7FFF
        P12- POWER SWITCH (MOMENTARY, ATX CASE SWITCH)
        JP1- RESET SWITCH (MOMENTARY, ATX CASE SWITCH)
        P16- BATTERY CONNECTION, P1=+ P2-4=-  (NON RECHARGEABLE)
        JP3- CTS FORCE HIGH
        P18- CONSOLE SERIAL CONNECTOR
        J11- TTL CONSOLE SERIAL CONNECTOR
        J12- ENABLE POWER TO TTL CONSOLE SERIAL CONNECTOR
        


### Default Memory Map
            $0000-$E000 - RAM
            $E000-$EFFF - Memory Mapped IO
            $F000-$FFFF - ROM
#### IO MEMORY MAP
            $E000-$EF7F - ISA IO SPACE
            $EF80-$EF8F - ACIA
            $EF90-$EF9F - RTC
            $EFA0-$EFAF - OPEN
            $EFB0-$EFBF - OPEN
            $EFC0-$EFCF - OPEN
            $EFD0-$EFDF - MMU TASK MAP
            $EFE0-$EFEF - MMU REGISTERS
            $EFF0-$EFFF - OPEN


