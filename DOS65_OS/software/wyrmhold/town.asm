;______________________________________________________________________________
;  town.asm - shop interaction inside towns
;
;  When the player presses USE ('T') in a town while adjacent to a
;  shop counter ('S' tile), the shop menu opens.  The menu is drawn
;  over the message rows; the player picks an option, gold is spent,
;  and the menu returns to the map.
;______________________________________________________________________________

; Eastmere's outfitter favors equipment; Valehaven's market favors
; healing and provisions. All services remain available in both.
shop_heal_price:
        .BYTE   15, 5
shop_food_price:
        .BYTE   25, 15
shop_food_amount:
        .BYTE   75, 150
shop_weapon_price:
        .BYTE   45, 75
shop_armor_price:
        .BYTE   35, 65
shop_potion_price:
        .BYTE   30, 12

;----------------------------------------------------------------
; use_action - bound to the USE key ('T'). Dispatch contextual
; interactions for the active location.
;----------------------------------------------------------------
use_action:
        LDA     loc
        CMP     #LOC_WORLD
        BEQ     @inworld
        CMP     #LOC_CASTLE
        BEQ     @incastle
        CMP     #LOC_TOWN
        BEQ     @intown
        JSR     sfx_blocked
        PRINTMSG_MSG t_nothing
        RTS
@inworld:
        JMP     overworld_use
@incastle:
        JMP     castle_use
@intown:
; is a shop tile adjacent (4-neighbour) or under us?
        JSR     shop_adjacent
        BCS     @open
        JMP     town_talk
@open:
        JMP     shop_menu

;----------------------------------------------------------------
; shop_adjacent - C=1 if a T_SHOP tile is at or next to the player.
;----------------------------------------------------------------
shop_adjacent:
; center
        LDA     px
        STA     tgtx
        LDA     py
        STA     tgty
        JSR     tileat
        LDA     tgttile
        CMP     #T_SHOP
        BEQ     @yes
; up
        LDA     px
        STA     tgtx
        LDA     py
        SEC
        SBC     #1
        STA     tgty
        JSR     tileat
        LDA     tgttile
        CMP     #T_SHOP
        BEQ     @yes
; down
        LDA     py
        CLC
        ADC     #1
        STA     tgty
        LDA     px
        STA     tgtx
        JSR     tileat
        LDA     tgttile
        CMP     #T_SHOP
        BEQ     @yes
; left
        LDA     py
        STA     tgty
        LDA     px
        SEC
        SBC     #1
        STA     tgtx
        JSR     tileat
        LDA     tgttile
        CMP     #T_SHOP
        BEQ     @yes
; right
        LDA     px
        CLC
        ADC     #1
        STA     tgtx
        JSR     tileat
        LDA     tgttile
        CMP     #T_SHOP
        BEQ     @yes
        CLC
        RTS
@yes:
        SEC
        RTS

;----------------------------------------------------------------
; town_talk - contextual town rumor/dialog.  Non-shop T use gives
; short guidance based on the active town and current quest state.
;----------------------------------------------------------------
town_talk:
        LDA     town_id
        ASL     A               ; town_id * 2
        STA     tmp0
        ASL     A               ; town_id * 4
        CLC
        ADC     tmp0            ; town_id * 6
        CLC
        ADC     queststate
        ASL     A
        TAX
        LDA     town_rumors,X
        LDY     town_rumors+1,X
        JSR     sfx_talk
        JSR     msg_print
        RTS

;----------------------------------------------------------------
; shop_menu - draw the shop menu over the message area and loop
; processing purchases until the player presses X.
;----------------------------------------------------------------
shop_menu:
        JSR     sfx_door
; start with the greeting as the status line
        JSR     shop_set_greeting
@redraw:
        JSR     shop_draw
@key:
        JSR     getkey_block
        CMP     #'1'
        BNE     :+
        JMP     @heal
:
        CMP     #'2'
        BNE     :+
        JMP     @food
:
        CMP     #'3'
        BNE     :+
        JMP     @weapon
:
        CMP     #'4'
        BNE     :+
        JMP     @armor
:
        CMP     #'5'
        BNE     :+
        JMP     @potion
:
        CMP     #'X'
        BNE     :+
        JMP     @goleave
:
        CMP     #'x'
        BNE     :+
        JMP     @goleave
:
        JSR     sfx_blocked
        SETSTAT t_badkey
        JMP     @redraw
@goleave:
        JMP     @leave
@heal:
        LDX     town_id
        LDA     shop_heal_price,X
        JSR     spend_gold
        BCS     :+
        JMP     @poor
:
        LDA     pmaxhp
        STA     phealth
        LDA     #0
        STA     poison_turns
        JSR     sfx_treasure
        SETSTAT t_healed
        JMP     @redraw
@food:
        LDX     town_id
        LDA     shop_food_price,X
        JSR     spend_gold
        BCS     :+
        JMP     @poor
:
        LDX     town_id
        CLC
        LDA     pfood
        ADC     shop_food_amount,X
        STA     pfood
        LDA     pfood+1
        ADC     #0
        STA     pfood+1
        JSR     sfx_treasure
        SETSTAT t_fed
        JMP     @redraw
@weapon:
        LDA     pweapon
        CMP     #3
        BCS     @maxwpn
        LDX     town_id
        LDA     shop_weapon_price,X
        JSR     spend_gold
        BCS     :+
        JMP     @poor
:
        INC     pweapon
        JSR     sfx_treasure
        SETSTAT t_boughtw
        JMP     @redraw
@maxwpn:
        JSR     sfx_blocked
        SETSTAT t_maxed
        JMP     @redraw
@armor:
        LDA     parmor
        CMP     #3
        BCS     @maxarm
        LDX     town_id
        LDA     shop_armor_price,X
        JSR     spend_gold
        BCS     :+
        JMP     @poor
:
        INC     parmor
        JSR     sfx_treasure
        SETSTAT t_boughta
        JMP     @redraw
@maxarm:
        JSR     sfx_blocked
        SETSTAT t_maxed
        JMP     @redraw
@potion:
        LDA     ppotions
        CMP     #POTION_MAX
        BCS     @maxpot
        LDX     town_id
        LDA     shop_potion_price,X
        JSR     spend_gold
        BCS     :+
        JMP     @poor
:
        INC     ppotions
        JSR     sfx_treasure
        SETSTAT t_boughtp
        JMP     @redraw
@maxpot:
        JSR     sfx_blocked
        SETSTAT t_potion_max
        JMP     @redraw
@poor:
        JSR     sfx_blocked
        SETSTAT t_poor
        JMP     @redraw
@leave:
; fully restore the game screen, then report leaving
        JSR     sfx_door
        JSR     full_redraw
        JSR     msg_redraw
        PRINTMSG_MSG t_bye
        RTS

;----------------------------------------------------------------
; spend_gold - try to spend A gold.  C=1 success (gold deducted),
;              C=0 if not enough.
;----------------------------------------------------------------
spend_gold:
        STA     tmp0
; gold >= tmp0 ?  (16-bit gold, 8-bit cost)
        LDA     pgold+1
        BNE     @ok             ; high byte nonzero -> plenty
        LDA     pgold
        CMP     tmp0
        BCC     @no
@ok:
        SEC
        LDA     pgold
        SBC     tmp0
        STA     pgold
        LDA     pgold+1
        SBC     #0
        STA     pgold+1
        SEC
        RTS
@no:
        CLC
        RTS

;----------------------------------------------------------------
; shop_set_greeting - select a greeting for the active town.
;----------------------------------------------------------------
shop_set_greeting:
        LDA     town_id
        ASL     A
        TAX
        LDA     shop_greetings,X
        STA     shopstat
        LDA     shop_greetings+1,X
        STA     shopstat+1
        RTS

;----------------------------------------------------------------
; shop_print_line - print a town-specific shop line.
;   IN: A = line index (SHOPTXT_*)
;----------------------------------------------------------------
SHOPTXT_HEADER  = 0
SHOPTXT_HEAL    = 1
SHOPTXT_FOOD    = 2
SHOPTXT_WEAPON  = 3
SHOPTXT_ARMOR   = 4
SHOPTXT_POTION  = 5

shop_print_line:
        ASL     A
        ASL     A               ; two town pointers per line
        STA     tmp0
        LDA     town_id
        ASL     A
        CLC
        ADC     tmp0
        TAX
        LDA     shop_text_table,X
        LDY     shop_text_table+1,X
        JMP     prmsg

;----------------------------------------------------------------
; Shop box geometry - a clean panel drawn inside the viewport,
; well clear of the bottom-right auto-scroll cell.
;----------------------------------------------------------------
SHOP_X0         = 3             ; left column of box
SHOP_X1         = 42            ; right column of box
SHOP_Y0         = 4             ; top row of box
SHOP_Y1         = 17            ; bottom row of box
SHOP_W          = SHOP_X1-SHOP_X0+1

;----------------------------------------------------------------
; shop_draw - render the shop as a self-contained bordered panel.
; The whole box is repainted each call (background + border via
; direct VRAM, then text via firmware) so it is always clean.
;----------------------------------------------------------------
shop_draw:
; --- paint box background + border directly to VRAM ---
        JSR     vid_enter
        LDA     #SHOP_Y0
        STA     rowidx
@bgrow:
        LDA     rowidx
        JSR     rowbase
        LDY     #SHOP_X0
@bgcol:
; border on edges, blank interior
        CPY     #SHOP_X0
        BEQ     @edge
        CPY     #SHOP_X1
        BEQ     @edge
        LDA     rowidx
        CMP     #SHOP_Y0
        BEQ     @edge
        CMP     #SHOP_Y1
        BEQ     @edge
; interior cell
        LDA     #space
        STA     (vptr),Y
        LDA     #C_SHOPBG
        STA     (cptr),Y
        JMP     @bgnext
@edge:
        LDA     #' '
        STA     (vptr),Y
        LDA     #C_SHOPBRD
        STA     (cptr),Y
@bgnext:
        INY
        CPY     #SHOP_X1+1
        BNE     @bgcol
        INC     rowidx
        LDA     rowidx
        CMP     #SHOP_Y1+1
        BNE     @bgrow
        JSR     vid_exit

; --- text lines via firmware (interior color C_SHOPBG) ---
        LDA     #C_SHOPTTL
        STA     CURCOLOR
        LDX     #SHOP_X0+8
        LDY     #SHOP_Y0
        JSR     locate
        LDA     #SHOPTXT_HEADER
        JSR     shop_print_line

        LDA     #C_SHOPTXT
        STA     CURCOLOR
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+2
        JSR     locate
        LDA     #SHOPTXT_HEAL
        JSR     shop_print_line
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+3
        JSR     locate
        LDA     #SHOPTXT_FOOD
        JSR     shop_print_line
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+4
        JSR     locate
        LDA     #SHOPTXT_WEAPON
        JSR     shop_print_line
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+5
        JSR     locate
        LDA     #SHOPTXT_ARMOR
        JSR     shop_print_line
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+6
        JSR     locate
        LDA     #SHOPTXT_POTION
        JSR     shop_print_line
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+7
        JSR     locate
        PRINTMSG t_optx
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+8
        JSR     locate
        JSR     shop_print_weapon_status

; gold line
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+9
        JSR     locate
        LDA     #C_SHOPTXT
        STA     CURCOLOR
        PRINTMSG t_gold
        COPY16  pgold, numarg
        JSR     displaynum
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y0+10
        JSR     locate
        JSR     shop_print_armor_status

; status line (last action / greeting)
        LDX     #SHOP_X0+3
        LDY     #SHOP_Y1-1
        JSR     locate
        LDA     #C_SHOPST
        STA     CURCOLOR
        LDA     shopstat
        LDY     shopstat+1
        JSR     prmsg
        JSR     draw_panel
        RTS

;----------------------------------------------------------------
; shop_print_weapon_status / shop_print_armor_status - show the
; current equipment and its actual gameplay trait in the shop.
;----------------------------------------------------------------
shop_print_weapon_status:
        PRINTMSG t_shop_weapon
        LDA     pweapon
        JSR     name_weapon
        LDA     strp
        LDY     strp+1
        JSR     prmsg
        PRINTMSG t_dash
        LDA     pweapon
        ASL     A
        TAX
        LDA     weapon_traits,X
        LDY     weapon_traits+1,X
        JMP     prmsg

shop_print_armor_status:
        PRINTMSG t_shop_armor
        LDA     parmor
        JSR     name_armor
        LDA     strp
        LDY     strp+1
        JSR     prmsg
        PRINTMSG t_dash
        LDA     parmor
        ASL     A
        TAX
        LDA     armor_traits,X
        LDY     armor_traits+1,X
        JMP     prmsg

;----------------------------------------------------------------
; Shop strings
;----------------------------------------------------------------
shop_text_table:
        .WORD   t_east_header, t_vale_header
        .WORD   t_east_heal, t_vale_heal
        .WORD   t_east_food, t_vale_food
        .WORD   t_east_weapon, t_vale_weapon
        .WORD   t_east_armor, t_vale_armor
        .WORD   t_east_potion, t_vale_potion
t_east_header:
        .BYTE   "- EASTMERE OUTFITTER -",0
t_vale_header:
        .BYTE   "- VALEHAVEN MARKET -",0
t_east_heal:
        .BYTE   "1) Heal to full     15 gold",0
t_vale_heal:
        .BYTE   "1) Heal to full      5 gold",0
t_east_food:
        .BYTE   "2) Provisions +75   25 gold",0
t_vale_food:
        .BYTE   "2) Provisions +150  15 gold",0
t_east_weapon:
        .BYTE   "3) Better weapon    45 gold",0
t_vale_weapon:
        .BYTE   "3) Better weapon    75 gold",0
t_east_armor:
        .BYTE   "4) Better armor     35 gold",0
t_vale_armor:
        .BYTE   "4) Better armor     65 gold",0
t_east_potion:
        .BYTE   "5) Healing potion   30 gold",0
t_vale_potion:
        .BYTE   "5) Healing potion   12 gold",0
t_optx:
        .BYTE   "X) Leave the shop",0
t_shop_weapon:
        .BYTE   "Weapon: ",0
t_dash:
        .BYTE   " - ",0
t_gold:
        .BYTE   "Thy gold: ",0
t_shop_armor:
        .BYTE   "Armor : ",0

weapon_traits:
        .WORD   wt0, wt1, wt2, wt3
wt0:
        .BYTE   "bare hands",0
wt1:
        .BYTE   "critical hits",0
wt2:
        .BYTE   "steady damage",0
wt3:
        .BYTE   "wide damage range",0

armor_traits:
        .WORD   at0, at1, at2, at3
at0:
        .BYTE   "no special defense",0
at1:
        .BYTE   "resists venom",0
at2:
        .BYTE   "balanced defense",0
at3:
        .BYTE   "best, eats food",0

town_rumors:
        .WORD   er0, er1, er2, er3, er4, er5
        .WORD   vr0, vr1, vr2, vr3, vr4, vr5
er0:
        .BYTE   "Eastmere gossip: the old road climbs toward Wyrmhold Castle.",0
er1:
        .BYTE   "Eastmere sailor: the Sunken Shrine waits in the southern marsh.",0
er2:
        .BYTE   "Eastmere armsman: take the Wyrm Key back to King Aldren.",0
er3:
        .BYTE   "Eastmere scout: the dragon cave lies northwest beyond the hills.",0
er4:
        .BYTE   "Eastmere bells ring: return to Aldren with word of victory.",0
er5:
        .BYTE   "Eastmere cheers the Champion of Wyrmhold.",0
vr0:
        .BYTE   "Valehaven trader: follow the road north, then east to the castle.",0
vr1:
        .BYTE   "Valehaven guide: leather helps in marsh and against serpent venom.",0
vr2:
        .BYTE   "Valehaven healer: Aldren can break the ward with the Wyrm Key.",0
vr3:
        .BYTE   "Valehaven elder: flooded halls guard the dragon's chamber.",0
vr4:
        .BYTE   "Valehaven market: the realm waits for the king's final word.",0
vr5:
        .BYTE   "Valehaven remembers the day the dragon fell.",0

shop_greetings:
        .WORD   t_east_greet, t_vale_greet
t_east_greet:
        .BYTE   "Eastmere arms the bold.",0
t_vale_greet:
        .BYTE   "Valehaven restores the weary.",0
t_nothing:
        .BYTE   "No one nearby answers.",0
t_noshop:
        .BYTE   "No merchant is close enough to answer.",0
t_healed:
        .BYTE   "Thou art fully healed.        ",0
t_fed:
        .BYTE   "Provisions purchased.         ",0
t_boughtw:
        .BYTE   "A finer weapon is thine!      ",0
t_boughta:
        .BYTE   "Sturdier armor is thine!      ",0
t_boughtp:
        .BYTE   "Healing potion purchased.     ",0
t_maxed:
        .BYTE   "Thou hast the finest already. ",0
t_potion_max:
        .BYTE   "Thy potion pouch is full.     ",0
t_poor:
        .BYTE   "Thou canst not afford that.   ",0
t_badkey:
        .BYTE   "Choose 1-5, or X to leave.   ",0
t_bye:
        .BYTE   "Come again, adventurer.",0
