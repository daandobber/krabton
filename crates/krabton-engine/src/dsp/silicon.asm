; Silicon: Krabton's own firmware for the DSP56300.
;
; Everything that sounds is worked out here, sample by sample, on the chip.
; First an LFO for the block. Then each voice: two oscillators that turn from
; a saw into a pulse of any width (the second frequency modulated by the
; first, or synced to it), ring modulation, a sub an octave under and noise,
; driven into a soft clip, a state variable filter (run twice a sample, so it
; opens far) and a second one after it or beside it, a filter and an
; amplifier envelope, the LFO on the pitch, the cutoff, the pulse widths and
; the level, and the voice placed in the stereo field. Then the effects on
; what the voices made: a chorus, a delay going from side to side, a reverb.
;
; Krabton is the host, as a microcontroller would be: it writes which voices
; play, their pitch, envelope targets and the knobs into X memory, starts the
; chip at render_block, waits for it at render_done, and reads the samples
; from Y memory.
;
; Copyright (C) Krabton contributors. GPL-3.0-only.

; ---- X memory, read every sample (short addresses)
G_SAW1          equ $00     ; the first oscillator: how much saw, how much pulse, the pulse's edge
G_SQ1           equ $01
G_PW1           equ $02
G_SAW2          equ $03     ; the second
G_SQ2           equ $04
G_PW2           equ $05
G_MIX2          equ $06     ; the second's level
G_SUB           equ $07
G_NOISE         equ $08
G_RING          equ $09
G_FM            equ $0a     ; how far the first moves the second's phase
G_DRIVE         equ $0b     ; the drive's gain, a sixteenth of it
G_ENVCUT        equ $0c     ; how far the filter envelope opens the filters (f/2, signed)
G_Q             equ $0d     ; filter 1: damping (q/2), and low, band, high pass mixed
G_CL            equ $0e
G_CB            equ $0f
G_CH            equ $10
G_Q2            equ $11     ; filter 2
G_C2L           equ $12
G_C2B           equ $13
G_C2H           equ $14
G_SER           equ $15     ; filter 2 takes filter 1's output
G_PAR           equ $16     ; ... or the mix
G_OUT1          equ $17     ; what goes out: filter 1, filter 2
G_OUT2          equ $18
G_LFO_PITCH     equ $19     ; the LFO on the pitch, the cutoff, the pulse widths, the level (half)
G_LFO_CUT       equ $1a
G_LFO_PW        equ $1b
G_LFO_AMP       equ $1c
G_FLAGS         equ $1d     ; bit 0: sync; bit 1: filter 2 on
G_RATIO2        equ $1e     ; filter 2's frequency against filter 1's, an eighth of it

; ---- X memory, read once a block (long addresses)
G_N             equ $40     ; samples to make (1 to 256)
G_VOICES        equ $41     ; voices to look at
LFO_PH          equ $42
LFO_INC         equ $43
LFO_TRI         equ $44     ; the LFO's shape: triangle, sine, saw and square mixed
LFO_SIN         equ $45
LFO_SAW         equ $46
LFO_SQ          equ $47
CH_MIX          equ $48     ; chorus
CH_BASE         equ $49     ; its delay, and how far it moves, in 2048ths of its line
CH_DEPTH        equ $4a
CH_PH           equ $4b
CH_INC          equ $4c
CH_POS          equ $4d     ; where it writes
CH_TRI          equ $4e
CH_FRAC         equ $4f
DL_MIX          equ $50     ; delay
DL_BACK         equ $51     ; how far back it reads (minus the delay)
DL_FB           equ $52
DL_TONE         equ $53
DL_LP_L         equ $54
DL_LP_R         equ $55
DL_POS_L        equ $56
DL_POS_R        equ $57
RV_MIX          equ $58     ; reverb
RV_FB           equ $59
RV_DAMP         equ $5a
RV_KEEP         equ $5b     ; 1 - damp
RV_POS_0        equ $5c     ; where its combs and all-passes are
RV_POS_1        equ $5d
RV_POS_2        equ $5e
RV_POS_3        equ $5f
RV_POS_4        equ $60
RV_POS_5        equ $61
RV_ST_0         equ $62     ; its combs' damping
RV_ST_1         equ $63
RV_ST_2         equ $64
RV_ST_3         equ $65
FX_IN           equ $66     ; a sample being worked on
FX_A            equ $67
FX_B            equ $68

; ---- a voice's 32 words: in X from V_BASE on, copied to S_BASE to be worked on
V_BASE          equ $100
S_BASE          equ $20
S_ACTIVE        equ $20     ; sounding (1) or not (0)
S_PH1           equ $21     ; phases (a whole cycle is -1 to 1) and how far they move a sample
S_INC1          equ $22
S_PH2           equ $23
S_INC2          equ $24
S_PHS           equ $25
S_INCS          equ $26
S_ENV           equ $27     ; the amplifier envelope (48 bits), where it goes and how fast
S_ENV_LO        equ $28
S_TARGET        equ $29
S_RATE          equ $2a
S_FENV          equ $2b     ; the filter envelope, the same
S_FENV_LO       equ $2c
S_FTARGET       equ $2d
S_FRATE         equ $2e
S_LOW           equ $2f     ; the filters' state
S_BAND          equ $30
S_LOW2          equ $31
S_BAND2         equ $32
S_VEL           equ $33     ; how loud
S_CUT           equ $34     ; the voice's own cutoff (f/2)
S_PAN_L         equ $35     ; where it is between the sides
S_PAN_R         equ $36
S_NOISE         equ $37     ; its noise (a shift register)
S_LFO           equ $39     ; worked out each sample
S_O1            equ $3a
S_VIB           equ $3b
S_F2            equ $3c
S_F1_OUT        equ $3d
S_F2_OUT        equ $3e

; ---- Y memory
OUT_L           equ $000    ; the samples made
OUT_R           equ $100
LFO_BUF         equ $200    ; the LFO, a sample at a time
CH_LINE_L       equ $1000   ; the chorus's lines (2048 long)
CH_LINE_R       equ $1800
DL_LINE_L       equ $10000  ; the delay's lines (32768 long)
DL_LINE_R       equ $18000

render_block:
        ; every address register counts plainly (the effects set their own)
        move    #>$ffffff,m0
        move    #>$ffffff,m1
        move    #>$ffffff,m2
        move    #>$ffffff,m3
        move    #>$ffffff,m4
        move    #>$ffffff,m5
        move    #>$ffffff,m6
        move    #>$ffffff,m7

        ; the output, cleared
        move    x:G_N,x0
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        clr     a
        do      x0,clear_end
        move    a,y:(r4)+
        move    a,y:(r5)+
clear_end:

        ; the LFO for the block: triangle, sine, saw and square as the host mixes them
        move    x:G_N,x0
        move    #>LFO_BUF,r3
        do      x0,lfo_end
        move    x:LFO_PH,a
        move    x:LFO_INC,x0
        add     x0,a
        move    a1,x:LFO_PH
        move    a1,x1                   ; x1 = the saw
        move    x:LFO_SAW,y0
        mpy     x1,y0,b
        move    x1,a
        asl     #8,a,a
        move    a,y1                    ; y1 = the square
        move    x:LFO_SQ,x0
        mac     x0,y1,b
        move    x1,a
        abs     a
        asl     a
        neg     a
        move    #>$7fffff,x0
        add     x0,a
        move    a,y1                    ; y1 = the triangle
        move    x:LFO_TRI,x0
        mac     x0,y1,b
        move    y1,a
        abs     a
        move    a,x0
        mpy     x0,y1,a                 ; t|t|
        neg     a
        add     y1,a
        add     y1,a                    ; 2t - t|t|: a sine, nearly
        move    a,x0
        move    x:LFO_SIN,y0
        mac     y0,x0,b
        move    b,y:(r3)+
lfo_end:

        ; the voices
        move    #>V_BASE,r0
        move    x:G_VOICES,x0
        do      x0,voices_end
        move    r0,r2                   ; the voice's place
        move    x:(r0),a
        tst     a
        jeq     voice_next              ; not sounding
        move    #S_BASE,r1
        do      #32,copy_in_end
        move    x:(r0)+,x0
        move    x0,x:(r1)+
copy_in_end:
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        move    #>LFO_BUF,r3
        move    x:G_N,x0
        do      x0,sample_end

        ; the LFO now, and what it does to the pitch
        move    y:(r3)+,x0
        move    x0,x:S_LFO
        move    x:G_LFO_PITCH,y0
        mpy     y0,x0,a
        move    a,x:S_VIB

        ; the first oscillator: its phase (osc 2 starting again when it comes round, if synced)
        move    x:S_VIB,y1
        move    x:S_INC1,x0
        move    x0,a
        mac     x0,y1,a
        move    a,x0
        move    x:S_PH1,a
        add     x0,a
        move    a1,x:S_PH1              ; wraps round (not limited)
        jec     osc1_round_done
        jclr    #0,x:G_FLAGS,osc1_round_done
        move    #>$800000,x0
        move    x0,x:S_PH2
osc1_round_done:
        ; its saw, and a pulse whose edge the LFO can move
        move    x:S_PH1,x1              ; x1 = the saw
        move    x:S_LFO,x0
        move    x:G_LFO_PW,y0
        move    x:G_PW1,b
        mac     y0,x0,b
        move    x1,a
        sub     b,a
        asl     #8,a,a
        move    a,y1                    ; y1 = the pulse
        move    x:G_SAW1,x0
        mpy     x1,x0,b
        move    x:G_SQ1,x0
        mac     x0,y1,b
        move    b,x:S_O1                ; osc 1

        ; the second: frequency modulated by the first
        move    x:S_VIB,y1
        move    x:S_INC2,x0
        move    x0,a
        mac     x0,y1,a
        move    a,x0
        move    x:S_PH2,a
        add     x0,a
        move    a1,x:S_PH2
        move    x:S_O1,x0
        move    x:G_FM,y0
        mac     y0,x0,a
        move    a1,x1                   ; x1 = its phase, moved
        move    x:S_LFO,x0
        move    x:G_LFO_PW,y0
        move    x:G_PW2,b
        mac     y0,x0,b
        move    x1,a
        sub     b,a
        asl     #8,a,a
        move    a,y1
        move    x:G_SAW2,x0
        mpy     x1,x0,b
        move    x:G_SQ2,x0
        mac     x0,y1,b
        move    b,x1                    ; x1 = osc 2

        ; the mix: osc 1, osc 2 at its level, the two rung together
        move    x:S_O1,a
        move    x:G_MIX2,x0
        mac     x1,x0,a
        move    x:S_O1,y0
        mpy     x1,y0,b
        move    b,y0
        move    x:G_RING,x0
        mac     y0,x0,a

        ; the sub, a square an octave under
        move    x:S_VIB,y1
        move    x:S_INCS,x0
        move    x0,b
        mac     x0,y1,b
        move    b,x0
        move    x:S_PHS,b
        add     x0,b
        move    b1,x:S_PHS
        move    b1,x1
        move    x1,b
        asl     #8,b,b
        move    b,x1
        move    x:G_SUB,x0
        mac     x1,x0,a

        ; noise: a shift register with feedback
        move    x:S_NOISE,b
        lsl     b
        jcc     noise_done
        move    #>$c20001,x0
        eor     x0,b
noise_done:
        move    b1,x:S_NOISE
        move    b1,x1
        move    x:G_NOISE,x0
        mac     x1,x0,a

        ; driven, and rounded off: 1.5x - 0.5x^3
        move    a,x0
        move    x:G_DRIVE,y0
        mpy     y0,x0,a
        asl     #4,a,a
        move    a,x0
        mpy     x0,x0,a
        move    a,y0
        mpy     y0,x0,a
        asr     a
        move    x0,b
        asr     b
        add     x0,b
        sub     a,b
        move    b,y1                    ; y1 = what goes into the filters

        ; the filter envelope: towards its target, 48 bits kept
        move    x:S_FENV,a
        move    x:S_FENV_LO,a0
        move    x:S_FTARGET,b
        sub     a,b
        move    b,x0
        move    x:S_FRATE,y0
        mac     y0,x0,a
        move    a1,x:S_FENV
        move    a0,x:S_FENV_LO

        ; filter 1's frequency: the voice's cutoff, the envelope, the LFO; kept where it is stable
        move    a,x0
        move    x:G_ENVCUT,y0
        move    x:S_CUT,a
        mac     y0,x0,a
        move    x:S_LFO,x0
        move    x:G_LFO_CUT,y0
        mac     y0,x0,a
        move    #>$3d70a4,b
        cmp     b,a
        tgt     b,a
        move    #>$001000,b
        cmp     b,a
        tlt     b,a
        move    a,x1                    ; x1 = f/2

        ; filter 2's, a ratio of it
        move    x:G_RATIO2,y0
        mpy     x1,y0,a
        asl     #3,a,a
        move    #>$3d70a4,b
        cmp     b,a
        tgt     b,a
        move    #>$001000,b
        cmp     b,a
        tlt     b,a
        move    a,x:S_F2

        ; filter 1: a state variable filter, twice
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
        move    x:G_CH,y0
        mpy     y0,x0,b
        move    x:S_LOW,x0
        move    x:G_CL,y0
        mac     y0,x0,b
        move    x:S_BAND,x0
        move    x:G_CB,y0
        mac     y0,x0,b
        move    b,x:S_F1_OUT

        ; filter 2 (when it is on): after filter 1, or beside it
        clr     b
        move    b,x:S_F2_OUT
        jclr    #1,x:G_FLAGS,filter2_done
        move    x:S_F1_OUT,x0
        move    x:G_SER,y0
        mpy     y0,x0,b
        move    y1,x0
        move    x:G_PAR,y0
        mac     y0,x0,b
        move    b,y1                    ; y1 = its input
        move    x:S_F2,x1
        move    x:S_BAND2,y0
        move    x:S_LOW2,a
        mac     x1,y0,a
        mac     x1,y0,a
        move    a,x:S_LOW2
        tfr     y1,b
        sub     a,b
        move    x:G_Q2,x0
        mac     -y0,x0,b
        mac     -y0,x0,b
        move    b,x0
        move    y0,a
        mac     x1,x0,a
        mac     x1,x0,a
        move    a,x:S_BAND2
        move    a,y0
        move    x:S_LOW2,a
        mac     x1,y0,a
        mac     x1,y0,a
        move    a,x:S_LOW2
        tfr     y1,b
        sub     a,b
        move    x:G_Q2,x0
        mac     -y0,x0,b
        mac     -y0,x0,b
        move    b,x0
        move    y0,a
        mac     x1,x0,a
        mac     x1,x0,a
        move    a,x:S_BAND2
        move    x:G_C2H,y0
        mpy     y0,x0,b
        move    x:S_LOW2,x0
        move    x:G_C2L,y0
        mac     y0,x0,b
        move    x:S_BAND2,x0
        move    x:G_C2B,y0
        mac     y0,x0,b
        move    b,x:S_F2_OUT
filter2_done:
        move    x:S_F1_OUT,x0
        move    x:G_OUT1,y0
        mpy     y0,x0,b
        move    x:S_F2_OUT,x0
        move    x:G_OUT2,y0
        mac     y0,x0,b
        move    b,x1                    ; x1 = filtered

        ; the amplifier: its envelope, the velocity, the LFO on the level
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
        move    a,x0                    ; x0 = the voice
        move    x:S_LFO,y0
        move    x:G_LFO_AMP,x1
        mpy     x1,y0,b
        neg     b
        sub     x1,b
        move    #>$7fffff,y0
        add     y0,b
        move    b,y0                    ; y0 = 1 - depth (1 + lfo) / 2
        mpy     y0,x0,a
        move    a,x0

        ; into the output, on its sides
        move    x:S_PAN_L,y0
        mpy     y0,x0,a
        move    y:(r4),b
        add     a,b
        move    b,y:(r4)+
        move    x:S_PAN_R,y0
        mpy     y0,x0,a
        move    y:(r5),b
        add     a,b
        move    b,y:(r5)+
sample_end:
        move    r2,r0
        move    #S_BASE,r1
        do      #32,copy_out_end
        move    x:(r1)+,x0
        move    x0,x:(r0)+
copy_out_end:
voice_next:
        move    r2,r0
        move    #32,n0
        nop
        lua     (r0)+n0,r0
voices_end:

        ; ---- chorus: each side read back from a line, the time moving (the other way on the right)
        move    x:CH_MIX,a
        tst     a
        jeq     chorus_done
        move    x:CH_POS,r6
        move    r6,a
        move    #>$800,x0
        add     x0,a
        move    a1,r7                   ; the right line, as far into it
        move    #>$7ff,m6
        move    #>$7ff,m7
        move    #>$7ff,m1
        move    #>$7ff,m2
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        move    x:G_N,x0
        do      x0,chorus_end
        move    x:CH_PH,a
        move    x:CH_INC,x0
        add     x0,a
        move    a1,x:CH_PH
        move    a1,x1
        move    x1,a
        abs     a
        asl     a
        neg     a
        move    #>$7fffff,x0
        add     x0,a
        move    a,x:CH_TRI              ; its triangle
        move    y:(r4),x0               ; the dry samples into the lines
        move    x0,y:(r6)
        move    y:(r5),x0
        move    x0,y:(r7)
        ; left
        move    x:CH_TRI,y1
        move    x:CH_DEPTH,y0
        move    x:CH_BASE,a
        mac     y1,y0,a
        move    r6,r1
        jsr     chorus_read
        move    y:(r4),b
        move    x:CH_MIX,y0
        mac     y0,x0,b
        move    b,y:(r4)+
        ; right, the time moving the other way
        move    x:CH_TRI,y1
        move    x:CH_DEPTH,y0
        move    x:CH_BASE,a
        mac     -y1,y0,a
        move    r7,r1
        jsr     chorus_read
        move    y:(r5),b
        move    x:CH_MIX,y0
        mac     y0,x0,b
        move    b,y:(r5)+
        lua     (r6)+,r6
        lua     (r7)+,r7
chorus_end:
        move    r6,x:CH_POS
        move    #>$ffffff,m6
        move    #>$ffffff,m7
        move    #>$ffffff,m1
        move    #>$ffffff,m2
chorus_done:

        ; ---- delay: the left line feeds the right and the right the left, darkened
        move    x:DL_MIX,a
        tst     a
        jeq     delay_done
        move    x:DL_POS_L,r6
        move    x:DL_POS_R,r7
        move    #>$7fff,m6
        move    #>$7fff,m7
        move    x:DL_BACK,n6
        move    x:DL_BACK,n7
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        move    x:G_N,x0
        do      x0,delay_end
        move    y:(r6+n6),x1            ; x1 = the left echo
        move    y:(r7+n7),y1            ; y1 = the right echo
        ; into the left line: the dry sound and the right echo fed back
        move    x:DL_FB,x0
        mpy     x0,y1,a
        move    a,x0
        move    x:DL_LP_L,a
        move    x:DL_TONE,y0
        move    x0,b
        sub     a,b
        move    b,x0
        mac     y0,x0,a
        move    a,x:DL_LP_L
        move    y:(r4),b
        move    y:(r5),x0
        add     x0,b
        asr     b
        add     a,b
        move    b,y:(r6)+
        ; into the right line: the left echo fed back
        move    x:DL_FB,x0
        mpy     x1,x0,a
        move    a,x0
        move    x:DL_LP_R,a
        move    x0,b
        sub     a,b
        move    b,x0
        mac     y0,x0,a
        move    a,x:DL_LP_R
        move    a,y:(r7)+
        ; the echoes out
        move    x:DL_MIX,y0
        move    y:(r4),a
        mac     x1,y0,a
        move    a,y:(r4)+
        move    y:(r5),a
        mac     y1,y0,a
        move    a,y:(r5)+
delay_end:
        move    r6,x:DL_POS_L
        move    r7,x:DL_POS_R
        move    #>$ffffff,m6
        move    #>$ffffff,m7
delay_done:

        ; ---- reverb: four combs side by side, two all-passes after them
        move    x:RV_MIX,a
        tst     a
        jeq     reverb_done
        move    x:RV_POS_0,r0
        move    x:RV_POS_1,r1
        move    x:RV_POS_2,r2
        move    x:RV_POS_3,r3
        move    x:RV_POS_4,r6
        move    x:RV_POS_5,r7
        move    #>1214,m0
        move    #>1292,m1
        move    #>1389,m2
        move    #>1475,m3
        move    #>604,m6
        move    #>479,m7
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        move    x:G_N,x0
        do      x0,reverb_end
        move    y:(r4),a
        move    y:(r5),x0
        add     x0,a
        asr     a
        asr     a
        asr     a
        move    a,x:FX_IN               ; in = (left + right) / 8
        clr     b
        move    b,x:FX_A

        move    y:(r0),x0
        move    x:RV_ST_0,a
        jsr     reverb_comb
        move    a,x:RV_ST_0
        move    b,y:(r0)+

        move    y:(r1),x0
        move    x:RV_ST_1,a
        jsr     reverb_comb
        move    a,x:RV_ST_1
        move    b,y:(r1)+

        move    y:(r2),x0
        move    x:RV_ST_2,a
        jsr     reverb_comb
        move    a,x:RV_ST_2
        move    b,y:(r2)+

        move    y:(r3),x0
        move    x:RV_ST_3,a
        jsr     reverb_comb
        move    a,x:RV_ST_3
        move    b,y:(r3)+

        ; the all-passes: out = held - in, held = in + held / 2
        move    x:FX_A,x1               ; the combs' sum
        move    y:(r6),x0
        move    x0,a
        sub     x1,a
        move    a,y1                    ; y1 = the first's output
        move    x0,a
        asr     a
        add     x1,a
        move    a,y:(r6)+
        move    y:(r7),x0
        move    x0,a
        sub     y1,a
        move    a,x1                    ; x1 = the second's output
        move    x0,a
        asr     a
        add     y1,a
        move    a,y:(r7)+
        ; out: the second on the left, the first on the right
        move    x:RV_MIX,y0
        move    y:(r4),a
        mac     x1,y0,a
        move    a,y:(r4)+
        move    y:(r5),a
        move    y1,x0
        mac     y0,x0,a
        move    a,y:(r5)+
reverb_end:
        move    r0,x:RV_POS_0
        move    r1,x:RV_POS_1
        move    r2,x:RV_POS_2
        move    r3,x:RV_POS_3
        move    r6,x:RV_POS_4
        move    r7,x:RV_POS_5
        move    #>$ffffff,m0
        move    #>$ffffff,m1
        move    #>$ffffff,m2
        move    #>$ffffff,m3
        move    #>$ffffff,m6
        move    #>$ffffff,m7
reverb_done:
render_done:
        jmp     render_done

; ---- the chorus's read: a = its time (in 2048ths of the line), r1 = where
; the line was written; x0 = the sample that far back, between two
chorus_read:
        asr     #12,a,a                 ; a1 = whole samples, a0 = the fraction
        move    a0,x0
        move    x0,b
        lsr     b
        move    b1,x:CH_FRAC            ; half the fraction, as a positive number
        move    a1,x0
        clr     b
        sub     x0,b
        move    b1,n1
        nop
        lua     (r1)+n1,r2              ; that many back
        nop
        move    y:(r2)-,y0              ; y0 = the newer of the two
        move    y:(r2),a                ; a = the older
        sub     y0,a
        move    a,x0
        move    x:CH_FRAC,y1
        move    y0,a
        mac     x0,y1,a
        mac     x0,y1,a
        move    a,x0
        rts

; ---- a comb: x0 = what it held, a = its damping's state; then a = the new
; state, b = what goes back into it
reverb_comb:
        move    x:FX_A,b
        add     x0,b
        move    b,x:FX_A                ; the combs' sum
        move    a,x1
        move    x:RV_KEEP,y0
        mpy     y0,x0,a
        move    x:RV_DAMP,y0
        mac     x1,y0,a
        move    a,x0
        move    x:RV_FB,y0
        mpy     y0,x0,b
        move    x:FX_IN,x1
        add     x1,b
        rts
