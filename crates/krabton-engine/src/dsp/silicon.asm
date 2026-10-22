; Silicon: Krabton's own firmware for the DSP56300.
;
; Everything that sounds is worked out here, sample by sample, on the chip:
; per voice a saw that turns into a square, a second saw beside it, a square
; an octave under, a state variable filter (run twice a sample, so it opens
; far) swept by its own envelope, and an amplifier envelope; the voices are
; added into the output. Krabton is the host, as a microcontroller would be:
; it writes which voices play, their pitch, envelope targets and the knobs
; into X memory, starts the chip at render_block, waits for it at
; render_done, and reads the samples from Y memory.
;
; Copyright (C) Krabton contributors. GPL-3.0-only.

; ---- X memory: what the host sets for a block
G_N             equ $00     ; samples to make (1 to 256)
G_SAWAMT        equ $01     ; how much saw (and square, below) the first oscillator has
G_SQAMT         equ $02
G_MIX           equ $03     ; the second saw's level
G_SUB           equ $04     ; the sub's level
G_ENVCUT        equ $05     ; how far the filter envelope opens the filter (f/2, signed)
G_Q             equ $06     ; the filter's damping (q/2)
G_CL            equ $07     ; the filter's output: low, band and high pass mixed
G_CB            equ $08
G_CH            equ $09
G_FDEC          equ $0a     ; the filter envelope's fall each sample
G_VOICES        equ $0b     ; voices to look at

; ---- a voice's sixteen words: in X from V_BASE on, copied to S_BASE to be worked on
V_BASE          equ $100
S_BASE          equ $20
S_ACTIVE        equ $20     ; sounding (1) or not (0)
S_PH1           equ $21     ; phases (a whole cycle is -1 to 1) and how far they move a sample
S_INC1          equ $22
S_PH2           equ $23
S_INC2          equ $24
S_PHS           equ $25
S_INCS          equ $26
S_ENV           equ $27     ; the amplifier envelope, 48 bits
S_ENV_LO        equ $28
S_TARGET        equ $29     ; where it goes, and how fast
S_RATE          equ $2a
S_FENV          equ $2b     ; the filter envelope
S_LOW           equ $2c     ; the filter's state
S_BAND          equ $2d
S_VEL           equ $2e     ; how loud
S_CUT           equ $2f     ; the voice's own cutoff (f/2)

; ---- Y memory: the samples made, from $000 on

render_block:
        move    x:G_N,x0
        move    #>$0,r4
        clr     a
        do      x0,clear_end
        move    a,y:(r4)+
clear_end:
        move    #>V_BASE,r0
        move    x:G_VOICES,x0
        do      x0,voices_end
        move    r0,r2                   ; the voice's place
        move    x:(r0),a
        tst     a
        jeq     voice_next              ; not sounding
        move    #S_BASE,r1
        do      #16,copy_in_end
        move    x:(r0)+,x0
        move    x0,x:(r1)+
copy_in_end:
        move    #>$0,r4
        move    x:G_N,x0
        do      x0,sample_end

        ; the first oscillator: a saw, and a square made from it
        move    x:S_PH1,a
        move    x:S_INC1,x0
        add     x0,a
        move    a1,x:S_PH1              ; wraps round (not limited)
        move    a1,x1                   ; x1 = the saw
        move    x1,a
        asl     #8,a,a
        move    a,y1                    ; y1 = the square (the saw, limited)
        move    x:G_SAWAMT,x0
        mpy     x1,x0,b
        move    x:G_SQAMT,x0
        mac     x0,y1,b

        ; the second: a saw
        move    x:S_PH2,a
        move    x:S_INC2,x0
        add     x0,a
        move    a1,x:S_PH2
        move    a1,x1
        move    x:G_MIX,x0
        mac     x1,x0,b

        ; the sub: a square an octave under
        move    x:S_PHS,a
        move    x:S_INCS,x0
        add     x0,a
        move    a1,x:S_PHS
        move    a1,x1
        move    x1,a
        asl     #8,a,a
        move    a,x1
        move    x:G_SUB,x0
        mac     x1,x0,b
        move    b,y1                    ; y1 = what goes into the filter

        ; the filter's frequency: the voice's cutoff, opened by its envelope
        move    x:S_FENV,x0
        move    x:G_FDEC,y0
        mpy     y0,x0,a
        move    a,x:S_FENV
        move    a,x0
        move    x:G_ENVCUT,y0
        move    x:S_CUT,a
        mac     y0,x0,a
        move    a,x1                    ; x1 = f/2

        ; the state variable filter, twice
        move    x:S_BAND,y0
        move    x:S_LOW,a
        mac     x1,y0,a
        mac     x1,y0,a                 ; low += f * band
        move    a,x:S_LOW
        tfr     y1,b
        sub     a,b
        move    x:G_Q,x0
        mac     -y0,x0,b
        mac     -y0,x0,b                ; high = in - low - q * band
        move    b,x0
        move    y0,a
        mac     x1,x0,a
        mac     x1,x0,a                 ; band += f * high
        move    a,x:S_BAND

        move    a,y0
        move    x:S_LOW,a
        mac     x1,y0,a
        mac     x1,y0,a
        move    a,x:S_LOW
        tfr     y1,b
        sub     a,b
        move    x:G_Q,x0
        mac     -y0,x0,b
        mac     -y0,x0,b
        move    b,x0                    ; x0 = high
        move    y0,a
        mac     x1,x0,a
        mac     x1,x0,a
        move    a,x:S_BAND

        ; what comes out of it: low, band and high pass as mixed
        move    x:G_CH,y0
        mpy     y0,x0,b
        move    x:S_LOW,x0
        move    x:G_CL,y0
        mac     y0,x0,b
        move    x:S_BAND,x0
        move    x:G_CB,y0
        mac     y0,x0,b
        move    b,x1                    ; x1 = filtered

        ; the amplifier: its envelope towards its target, kept to 48 bits
        move    x:S_ENV,a
        move    x:S_ENV_LO,a0
        move    x:S_TARGET,b
        sub     a,b
        move    b,x0
        move    x:S_RATE,y0
        mac     y0,x0,a
        move    a1,x:S_ENV
        move    a0,x:S_ENV_LO
        move    a,y0
        mpy     x1,y0,a
        move    a,x0
        move    x:S_VEL,y0
        mpy     y0,x0,a

        ; into the output
        move    y:(r4),b
        add     a,b
        move    b,y:(r4)+
sample_end:
        move    r2,r0
        move    #S_BASE,r1
        do      #16,copy_out_end
        move    x:(r1)+,x0
        move    x0,x:(r0)+
copy_out_end:
voice_next:
        move    r2,r0
        move    #16,n0
        nop
        lua     (r0)+n0,r0
voices_end:
render_done:
        jmp     render_done
