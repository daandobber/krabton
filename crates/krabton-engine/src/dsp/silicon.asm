; Silicon: Krabton's own firmware for the DSP56300.
;
; Everything that sounds is worked out here, sample by sample, on the chip.
; Two LFOs for the block (triangle, sine, saw, square, sample and hold,
; sample and glide). Then each voice: two oscillators, each a classic wave
; (a saw turning into a pulse of any width), a hypersaw (up to nine saws
; spread around the note) or a wavetable (two waves of a table morphed);
; the second frequency modulated by the first or synced to it, its pitch
; and its FM swept by the filter envelope; a third oscillator, a sub (square
; or triangle), coloured noise, ring modulation; a drive into a soft clip;
; filter 1 (a state variable filter run twice a sample, or a four pole
; ladder), saturation after it, filter 2 after it, beside it, or each
; oscillator through its own; a filter and an amplifier envelope; the LFOs
; on the pitch, the cutoffs, the pulse widths, the FM, the wave, the level
; and the pan. Then the effects on what the voices made: distortion (also
; fewer bits or a lower rate), a chorus or a flanger, a phaser, a delay going
; from side to side, a reverb, and a boost of the low end.
;
; Krabton is the host, as a microcontroller would be: it writes which voices
; play, their pitch, envelope targets and the knobs into X memory (and the
; wavetables into Y memory once), starts the chip at render_block, waits for
; it at render_done, and reads the samples from Y memory.
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
G_NOISE         equ $08     ; the noise: white, and (below) low passed
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
G_LFO_PITCH     equ $19     ; LFO 1 on the pitch, the cutoff, the pulse widths, the level (half)
G_LFO_CUT       equ $1a
G_LFO_PW        equ $1b
G_LFO_AMP       equ $1c
G_FLAGS         equ $1d     ; 0 sync, 1 filter 2 on, 2 osc 1 hypersaw, 3 osc 1 wavetable,
                            ; 4 osc 2 hypersaw, 5 osc 2 wavetable, 6 sub triangle, 7 ladder, 8 split
G_RATIO2        equ $1e     ; filter 2's frequency against filter 1's, an eighth of it
G_FLAGS2        equ $1f     ; saturation: 0 light, 1 soft, 2 hard, 3 digital, 4 shaper, 5 rectifier

; ---- X memory, read every sample (long addresses)
G_SPLIT         equ $88     ; filter 2 takes the second oscillators' mix (split)
G_MIX3          equ $89     ; the third oscillator
G_SAW3          equ $8a
G_SQ3           equ $8b
G_PW3           equ $8c
G_NOISE_LP      equ $8d     ; the noise's low pass, and how much of it
G_NOISE_DARK    equ $8e
G_ENV_OSC2      equ $8f     ; the filter envelope on osc 2's pitch, and on the FM
G_ENV_FM        equ $90
G_SAT_GAIN      equ $91     ; the saturation's gain, a sixteenth of it
G_LAD_K         equ $92     ; the ladder's resonance, a quarter of it
G_LAD_C2        equ $93     ; the ladder's output: its second and its fourth stage
G_LAD_C4        equ $94
G_LFO2_CUT2     equ $95     ; LFO 2 on filter 2's cutoff, the pan, the FM, the wave
G_LFO2_PAN      equ $96
G_LFO2_FM       equ $97
G_LFO2_WAVE     equ $98
G_WT_POS1       equ $99     ; where in their tables the oscillators are (0 to 1)
G_WT_POS2       equ $9a
G_WT_BASE1      equ $9b     ; their tables, in Y memory
G_WT_BASE2      equ $9c
G_HS_N          equ $9d     ; the hypersaw: how many saws
G_HS_TAB        equ $a0     ; ... and each one's detune and level (18 words)

; ---- X memory, read once a block (long addresses)
G_N             equ $40     ; samples to make (1 to 256)
G_VOICES        equ $41     ; voices to look at
CH_MIX          equ $48     ; chorus (or flanger)
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
CH_FB           equ $69     ; the flanger's feedback, and what it fed back
CH_WET_L        equ $6a
CH_WET_R        equ $6b
LFO1            equ $70     ; the LFOs: 11 words each
LFO2            equ $7b
RNG             equ $86     ; the chip's own randomness
L_PH            equ 0       ; ... an LFO's words: its phase, how far it moves a sample,
L_INC           equ 1       ; its shapes mixed, what it holds and how it glides there
L_TRI           equ 2
L_SIN           equ 3
L_SAW           equ 4
L_SQ            equ 5
L_SH            equ 6
L_SG            equ 7
L_HELD          equ 8
L_GLIDE         equ 9
L_GLIDE_RATE    equ 10
FX_FLAGS        equ $100    ; 0 light, 1 soft, 2 hard, 3 bits, 4 rate (distortion)
DS_GAIN         equ $101    ; its gain, a sixteenth of it
DS_STEP         equ $102    ; its steps (bits)
DS_RATE         equ $103    ; its rate (rate reducer), how far it is, what it holds
DS_ACC          equ $104
DS_HOLD_L       equ $105
DS_HOLD_R       equ $106
PH_MIX          equ $108    ; phaser
PH_PH           equ $109
PH_INC          equ $10a
PH_FB           equ $10b
PH_BASE         equ $10c
PH_DEPTH        equ $10d
PH_LAST_L       equ $10e
PH_LAST_R       equ $10f
PH_COEF         equ $110
PH_STAGES_L     equ $112    ; six stages, two words each
PH_STAGES_R     equ $120
BO_AMT          equ $130    ; analog boost
BO_COEF         equ $131
BO_L            equ $132
BO_R            equ $133
FX_SCRATCH      equ $20     ; a short address for the effects' flags (the voices are done)

; ---- a voice's 64 words: in X from V_BASE on; the first 32 copied to S_BASE,
; the next 32 to S_BASE_B, to be worked on
V_BASE          equ $200
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
S_BASE_B        equ $c0
S_HS1           equ $c0     ; the hypersaws' phases (nine each)
S_HS2           equ $c9
S_PH3           equ $d2     ; the third oscillator
S_INC3          equ $d3
S_LAD1          equ $d4     ; the ladder's stages
S_LAD2          equ $d5
S_LAD3          equ $d6
S_LAD4          equ $d7
S_NCOL          equ $d8     ; the noise's low pass
S_INC1V         equ $e0     ; worked out each sample (not kept)
S_INC2V         equ $e1
S_INC3V         equ $e2
S_LFO2          equ $e3
S_MIX_A         equ $e4
S_MIX_B         equ $e5
S_IN            equ $e6
S_G             equ $e7
S_FMAMT         equ $e8
S_TR_U          equ $e9
S_TR_POS        equ $ea
S_TR_BASE       equ $eb
S_TR_A          equ $ec
S_TR_FRAC       equ $ed
S_O2            equ $ee

; ---- Y memory
OUT_L           equ $000    ; the samples made
OUT_R           equ $100
LFO_BUF         equ $200    ; LFO 1, a sample at a time (LFO 2 the next 256)
LFO2_BUF        equ $300
CH_LINE_L       equ $1000   ; the chorus's lines (2048 long)
CH_LINE_R       equ $1800
DL_LINE_L       equ $10000  ; the delay's lines (32768 long)
DL_LINE_R       equ $18000

render_block:
        ; every address register counts plainly (the effects and the tables set their own)
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

        ; the LFOs for the block
        move    #>LFO1,r6
        move    #>LFO_BUF,r3
        jsr     lfo_block
        move    #>LFO2,r6
        move    #>LFO2_BUF,r3
        jsr     lfo_block

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
        move    #>S_BASE_B,r1
        do      #32,copy_in_b_end
        move    x:(r0)+,x0
        move    x0,x:(r1)+
copy_in_b_end:
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        move    #>LFO_BUF,r3
        move    #>$100,n3
        move    x:G_N,x0
        do      x0,sample_end
        jsr     voice_sample
        nop
sample_end:
        move    r2,r0
        move    #S_BASE,r1
        do      #32,copy_out_end
        move    x:(r1)+,x0
        move    x0,x:(r0)+
copy_out_end:
        move    #>S_BASE_B,r1
        do      #32,copy_out_b_end
        move    x:(r1)+,x0
        move    x0,x:(r0)+
copy_out_b_end:
voice_next:
        move    r2,r0
        move    #>64,n0
        nop
        lua     (r0)+n0,r0
voices_end:

        jsr     fx_distortion
        jsr     fx_chorus
        jsr     fx_phaser
        jsr     fx_delay
        jsr     fx_reverb
        jsr     fx_boost
render_done:
        jmp     render_done

; ==== an LFO for the block: r6 = its words, r3 = where its samples go
lfo_block:
        move    x:G_N,x0
        do      x0,lfo_end
        move    #>L_INC,n6
        nop
        move    x:(r6+n6),x0
        move    x:(r6),a
        add     x0,a
        move    a1,x:(r6)               ; its phase
        jec     lfo_hold_done
        move    x:RNG,b                 ; round again: something new to hold
        lsl     b
        jcc     lfo_rng_done
        move    #>$c20001,x0
        eor     x0,b
lfo_rng_done:
        move    b1,x:RNG
        move    #>L_HELD,n6
        nop
        move    b1,x:(r6+n6)
lfo_hold_done:
        move    x:(r6),x1               ; x1 = the saw
        move    #>L_SAW,n6
        nop
        move    x:(r6+n6),y0
        mpy     x1,y0,b
        move    x1,a
        asl     #8,a,a
        move    a,y1                    ; the square
        move    #>L_SQ,n6
        nop
        move    x:(r6+n6),x0
        mac     x0,y1,b
        move    x1,a
        abs     a
        asl     a
        neg     a
        move    #>$7fffff,x0
        add     x0,a
        move    a,y1                    ; the triangle
        move    #>L_TRI,n6
        nop
        move    x:(r6+n6),x0
        mac     x0,y1,b
        move    y1,a
        abs     a
        move    a,x0
        mpy     x0,y1,a                 ; t|t|
        neg     a
        add     y1,a
        add     y1,a                    ; 2t - t|t|: a sine, nearly
        move    a,x0
        move    #>L_SIN,n6
        nop
        move    x:(r6+n6),y0
        mac     y0,x0,b
        move    #>L_HELD,n6             ; what it holds
        nop
        move    x:(r6+n6),x1
        move    #>L_SH,n6
        nop
        move    x:(r6+n6),y0
        mac     x1,y0,b
        move    b,x:FX_B
        move    #>L_GLIDE,n6            ; and gliding there
        nop
        move    x:(r6+n6),a
        tfr     x1,b
        sub     a,b
        move    b,x0
        move    #>L_GLIDE_RATE,n6
        nop
        move    x:(r6+n6),y0
        mac     y0,x0,a
        move    #>L_GLIDE,n6
        nop
        move    a,x:(r6+n6)
        move    a,x0
        move    #>L_SG,n6
        nop
        move    x:(r6+n6),y0
        move    x:FX_B,b
        mac     y0,x0,b
        move    b,y:(r3)+
lfo_end:
        rts

; ==== one sample of one voice
voice_sample:
        ; the LFOs now
        move    y:(r3+n3),x1
        move    x1,x:S_LFO2
        move    y:(r3)+,x0
        move    x0,x:S_LFO
        move    x:G_LFO_PITCH,y0
        mpy     y0,x0,a
        move    a,x:S_VIB

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

        ; the increments, moved by the LFO (and osc 2's by the envelope)
        move    x:S_VIB,y1
        move    x:S_INC1,x0
        move    x0,a
        mac     x0,y1,a
        move    a,x:S_INC1V
        move    x:S_INC3,x0
        move    x0,a
        mac     x0,y1,a
        move    a,x:S_INC3V
        move    x:S_FENV,x0
        move    x:G_ENV_OSC2,y0
        mpy     y0,x0,a
        move    x:S_VIB,b
        add     a,b
        move    b,y1                    ; vibrato + envelope
        move    x:S_INC2,x0
        move    x0,a
        mac     x0,y1,a
        move    a,x:S_INC2V
        ; the FM's amount: the knob, the envelope, LFO 2
        move    x:G_FM,a
        move    x:S_FENV,x0
        move    x:G_ENV_FM,y0
        mac     y0,x0,a
        move    x:S_LFO2,x0
        move    x:G_LFO2_FM,y0
        mac     y0,x0,a
        move    a,x:S_FMAMT

        ; ---- osc 1: its phase first (osc 2 starting again when it comes round, if synced)
        move    x:S_INC1V,x0
        move    x:S_PH1,a
        add     x0,a
        move    a1,x:S_PH1              ; wraps round (not limited)
        jec     osc1_round_done
        jclr    #0,x:G_FLAGS,osc1_round_done
        move    #>$800000,x0
        move    x0,x:S_PH2
osc1_round_done:
        jset    #2,x:G_FLAGS,osc1_hyper
        jset    #3,x:G_FLAGS,osc1_table
        move    x:S_PH1,x1              ; classic: the saw, and a pulse
        move    x:S_LFO,x0
        move    x:G_LFO_PW,y0
        move    x:G_PW1,b
        mac     y0,x0,b
        move    x1,a
        sub     b,a
        asl     #8,a,a
        move    a,y1
        move    x:G_SAW1,x0
        mpy     x1,x0,b
        move    x:G_SQ1,x0
        mac     x0,y1,b
        jmp     osc1_done
osc1_hyper:
        move    #>S_HS1,r6
        move    x:S_INC1V,y1
        jsr     hyper_saw
        jmp     osc1_done
osc1_table:
        move    x:S_PH1,a
        move    x:G_WT_POS1,b
        move    x:G_WT_BASE1,x0
        move    x0,x:S_TR_BASE
        jsr     table_read
osc1_done:
        move    b,x:S_O1

        ; ---- osc 2: frequency modulated by osc 1
        move    x:S_INC2V,x0
        move    x:S_PH2,a
        add     x0,a
        move    a1,x:S_PH2
        move    x:S_O1,x0
        move    x:S_FMAMT,y0
        mac     y0,x0,a
        move    a1,x:S_TR_A             ; its phase, moved
        jset    #4,x:G_FLAGS,osc2_hyper
        jset    #5,x:G_FLAGS,osc2_table
        move    x:S_TR_A,x1
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
        jmp     osc2_done
osc2_hyper:
        move    #>S_HS2,r6
        move    x:S_INC2V,y1
        jsr     hyper_saw
        jmp     osc2_done
osc2_table:
        move    x:S_TR_A,a
        move    x:G_WT_POS2,b
        move    x:G_WT_BASE2,x0
        move    x0,x:S_TR_BASE
        jsr     table_read
osc2_done:
        move    b,x:S_O2

        ; ---- osc 3: a saw turning into a pulse
        move    x:S_INC3V,x0
        move    x:S_PH3,a
        add     x0,a
        move    a1,x:S_PH3
        move    a1,x1
        move    x:G_PW3,b
        move    x1,a
        sub     b,a
        asl     #8,a,a
        move    a,y1
        move    x:G_SAW3,x0
        mpy     x1,x0,b
        move    x:G_SQ3,x0
        mac     x0,y1,b
        move    b,x1                    ; x1 = osc 3

        ; the second mix: osc 2 at its level, osc 3 at its, the ring
        move    x:S_O2,y1
        move    x:G_MIX2,x0
        mpy     x0,y1,a
        move    x:G_MIX3,x0
        mac     x1,x0,a
        move    x:S_O1,x0
        mpy     x0,y1,b                 ; osc 1 * osc 2
        move    b,x0
        move    x:G_RING,y0
        mac     y0,x0,a
        move    a,x:S_MIX_B

        ; the first: osc 1, the sub (square or triangle), the noise
        move    x:S_O1,a
        move    x:S_INCS,x0
        move    x0,b
        move    x:S_VIB,y1
        mac     x0,y1,b
        move    b,x0
        move    x:S_PHS,b
        add     x0,b
        move    b1,x:S_PHS
        move    b1,x1
        jset    #6,x:G_FLAGS,sub_triangle
        move    x1,b
        asl     #8,b,b
        move    b,x1
        jmp     sub_done
sub_triangle:
        move    x1,b
        abs     b
        asl     b
        neg     b
        move    #>$7fffff,x0
        add     x0,b
        move    b,x1
sub_done:
        move    x:G_SUB,x0
        mac     x1,x0,a
        move    x:S_NOISE,b             ; noise: a shift register with feedback
        lsl     b
        jcc     noise_done
        move    #>$c20001,x0
        eor     x0,b
noise_done:
        move    b1,x:S_NOISE
        move    b1,x1                   ; x1 = white
        move    x:G_NOISE,x0
        mac     x1,x0,a
        move    a,x:S_MIX_A
        move    x:S_NCOL,a              ; and low passed
        move    x1,b
        sub     a,b
        move    b,x0
        move    x:G_NOISE_LP,y0
        mac     y0,x0,a
        move    a,x:S_NCOL
        move    a,x0
        move    x:G_NOISE_DARK,y0
        move    x:S_MIX_A,a
        mac     y0,x0,a
        move    a,x:S_MIX_A

        ; what goes into filter 1 (split: only the first mix), driven and rounded off
        move    x:S_MIX_A,a
        jset    #8,x:G_FLAGS,in_split
        move    x:S_MIX_B,x0
        add     x0,a
in_split:
        move    a,x0
        move    x:G_DRIVE,y0
        mpy     y0,x0,a
        asl     #4,a,a
        move    a,x0
        jsr     soft_clip
        move    b,x:S_IN

        ; filter 1's frequency: the voice's cutoff, the envelope, LFO 1; kept where it is stable
        move    x:S_FENV,x0
        move    x:G_ENVCUT,y0
        move    x:S_CUT,a
        mac     y0,x0,a
        move    x:S_LFO,x0
        move    x:G_LFO_CUT,y0
        mac     y0,x0,a
        jsr     clamp_f
        move    a,x1                    ; x1 = f/2
        ; filter 2's, a ratio of it, moved by LFO 2
        move    x:G_RATIO2,y0
        mpy     x1,y0,a
        asl     #3,a,a
        move    a,x0
        move    x:S_LFO2,y0
        move    x:G_LFO2_CUT2,y1
        mpy     y1,y0,b
        add     x0,b
        tfr     b,a
        jsr     clamp_f
        move    a,x:S_F2

        ; ---- filter 1: the ladder, or a state variable filter twice
        jset    #7,x:G_FLAGS,filter1_ladder
        move    x:S_IN,y1
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
        jmp     filter1_done
filter1_ladder:
        ; four one-poles (g = 4 f/2, at most 0.9), the fourth fed back, the input rounded off
        move    x1,a
        asl     #2,a,a
        move    #>$733333,b
        cmp     b,a
        tgt     b,a
        move    a,x:S_G
        move    x:S_LAD4,x0
        move    x:G_LAD_K,y0
        mpy     y0,x0,a
        asl     #2,a,a
        move    x:S_IN,b
        sub     a,b
        move    b,x0
        jsr     soft_clip
        move    b,x1
        move    x:S_G,y1
        move    x:S_LAD1,a
        move    x1,b
        sub     a,b
        move    b,x0
        mac     x0,y1,a
        move    a,x:S_LAD1
        move    a,x1
        move    x:S_LAD2,a
        move    x1,b
        sub     a,b
        move    b,x0
        mac     x0,y1,a
        move    a,x:S_LAD2
        move    a,x1
        move    x:S_LAD3,a
        move    x1,b
        sub     a,b
        move    b,x0
        mac     x0,y1,a
        move    a,x:S_LAD3
        move    a,x1
        move    x:S_LAD4,a
        move    x1,b
        sub     a,b
        move    b,x0
        mac     x0,y1,a
        move    a,x:S_LAD4
        move    x:S_LAD2,x0
        move    x:G_LAD_C2,y0
        mpy     y0,x0,b
        move    x:S_LAD4,x0
        move    x:G_LAD_C4,y0
        mac     y0,x0,b
        move    b,x:S_F1_OUT
filter1_done:

        ; ---- saturation after filter 1
        move    x:G_FLAGS2,a
        tst     a
        jeq     saturation_done
        move    x:S_F1_OUT,x0
        move    x:G_SAT_GAIN,y0
        mpy     y0,x0,a
        asl     #4,a,a
        move    a,x0                    ; x0 = driven, limited
        jset    #0,x:G_FLAGS2,sat_light
        jset    #1,x:G_FLAGS2,sat_soft
        jset    #2,x:G_FLAGS2,sat_hard
        jset    #3,x:G_FLAGS2,sat_digital
        jset    #4,x:G_FLAGS2,sat_shaper
        move    x0,a                    ; rectifier: both halves up
        abs     a
        move    a,x:S_F1_OUT
        jmp     saturation_done
sat_light:
        move    x0,a
        asr     a
        move    a,x0
        jsr     soft_clip
        asl     b
        move    b,x:S_F1_OUT
        jmp     saturation_done
sat_soft:
        jsr     soft_clip
        move    b,x:S_F1_OUT
        jmp     saturation_done
sat_hard:
        move    x0,a
        asl     #2,a,a
        move    a,x:S_F1_OUT            ; limited: clipped hard
        jmp     saturation_done
sat_digital:
        move    x0,a                    ; a few bits
        move    #>$ffc000,x1
        and     x1,a
        move    a1,x:S_F1_OUT
        jmp     saturation_done
sat_shaper:
        mpy     x0,x0,a                 ; 4x^3 - 3x: folds over
        move    a,y0
        mpy     y0,x0,a
        asl     #2,a,a
        move    x0,b
        asl     b
        add     x0,b
        sub     b,a
        move    a,x:S_F1_OUT
saturation_done:

        ; ---- filter 2 (when it is on): after filter 1, beside it, or on the second mix
        clr     b
        move    b,x:S_F2_OUT
        jclr    #1,x:G_FLAGS,filter2_done
        move    x:S_F1_OUT,x0
        move    x:G_SER,y0
        mpy     y0,x0,b
        move    x:S_IN,x0
        move    x:G_PAR,y0
        mac     y0,x0,b
        move    x:S_MIX_B,x0
        move    x:G_SPLIT,y0
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

        ; ---- the amplifier: its envelope, the velocity, LFO 1 on the level
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

        ; ---- into the output, on its sides (LFO 2 moving it across)
        move    x:S_LFO2,y0
        move    x:G_LFO2_PAN,x1
        mpy     x1,y0,b
        move    b,y1                    ; y1 = how far LFO 2 moves it
        move    x:S_PAN_L,a
        move    x:S_PAN_L,y0
        mac     -y1,y0,a
        move    a,y0
        mpy     y0,x0,a
        move    y:(r4),b
        add     a,b
        move    b,y:(r4)+
        move    x:S_PAN_R,a
        move    x:S_PAN_R,y0
        mac     y1,y0,a
        move    a,y0
        mpy     y0,x0,a
        move    y:(r5),b
        add     a,b
        move    b,y:(r5)+
        rts

; ---- a hypersaw: r6 = its phases, y1 = the increment; b = the saws, each detuned and at its level
hyper_saw:
        clr     b
        move    #>G_HS_TAB,r7
        move    x:G_HS_N,x0
        do      x0,hypersaw_end
        move    x:(r7)+,x0              ; its detune (a fraction of the increment)
        mpy     x0,y1,a
        add     y1,a
        move    a,x0
        move    x:(r6),a
        add     x0,a
        move    a1,x:(r6)+
        move    a1,x1
        move    x:(r7)+,y0              ; its level
        mac     x1,y0,b
hypersaw_end:
        rts

; ---- a wavetable: a = the phase, b = where in the table (0 to 1), x:S_TR_BASE = the
; table in Y memory (8 waves of 256); b = the two waves it is between, morphed
table_read:
        move    #>$800000,x0
        eor     x0,a
        move    a1,x:S_TR_U             ; the phase from 0
        move    b,x0                    ; the position, moved by LFO 2
        move    x:S_LFO2,y0
        move    x:G_LFO2_WAVE,y1
        mpy     y1,y0,b
        add     x0,b
        move    #>$000001,x0
        cmp     x0,b
        tlt     x0,b
        move    b,x0
        move    #>$700000,y0
        mpy     y0,x0,b                 ; position * 7/8: whole waves in its top bits
        move    b1,a
        lsr     #20,a
        move    a1,x1                   ; x1 = the wave (0 to 6)
        move    #>$0fffff,x0
        and     x0,b
        lsl     #3,b
        move    b1,x:S_TR_FRAC          ; the morph to the next
        move    x1,a
        lsl     #8,a
        move    x:S_TR_BASE,x0
        add     x0,a
        move    x:S_TR_U,b
        lsr     #16,b
        move    b1,x0
        add     x0,a
        move    a1,r6                   ; the first wave, at the sample
        move    #>$100,x0
        add     x0,a
        move    a1,r7                   ; the second
        move    #>$ff,m6
        move    #>$ff,m7
        move    x:S_TR_U,b              ; between the samples: the rest of the phase
        move    #>$00ffff,x0
        and     x0,b
        lsl     #7,b
        move    b1,y1                   ; y1 = how far to the next sample
        move    y:(r6)+,y0
        move    y:(r6),a
        sub     y0,a
        move    a,x0
        move    y0,a
        mac     x0,y1,a
        move    a,x:S_TR_A              ; the first wave there
        move    y:(r7)+,y0
        move    y:(r7),a
        sub     y0,a
        move    a,x0
        move    y0,b
        mac     x0,y1,b                 ; the second
        move    x:S_TR_A,a
        sub     a,b
        move    b,x0
        move    x:S_TR_FRAC,y0
        mac     y0,x0,a                 ; morphed
        tfr     a,b
        move    #>$ffffff,m6
        move    #>$ffffff,m7
        rts

; ---- 1.5x - 0.5x^3 of x0 (limited), into b
soft_clip:
        mpy     x0,x0,a
        move    a,y0
        mpy     y0,x0,a
        asr     a
        move    x0,b
        asr     b
        add     x0,b
        sub     a,b
        rts

; ---- a filter's f/2 (in a) kept between a little and 0.48
clamp_f:
        move    #>$3d70a4,b
        cmp     b,a
        tgt     b,a
        move    #>$001000,b
        cmp     b,a
        tlt     b,a
        rts

; ==== distortion: on both sides, as the host chose
fx_distortion:
        move    x:FX_FLAGS,x0
        move    x0,x:FX_SCRATCH
        move    x0,a
        tst     a
        jeq     fx_distortion_end
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        move    x:G_N,x0
        do      x0,distortion_loop
        move    y:(r4),x0
        jsr     distort_one
        move    a,y:(r4)+
        move    y:(r5),x0
        jsr     distort_one
        move    a,y:(r5)+
        ; the rate reducer: a new sample only when its clock comes round
        jclr    #4,x:FX_SCRATCH,rate_done
        move    x:DS_ACC,a
        move    x:DS_RATE,x0
        add     x0,a
        move    a1,x:DS_ACC
        jec     rate_hold
        move    y:-(r4),x0
        move    x0,x:DS_HOLD_L
        move    y:(r4)+,x0
        move    y:-(r5),x0
        move    x0,x:DS_HOLD_R
        move    y:(r5)+,x0
        jmp     rate_done
rate_hold:
        move    x:DS_HOLD_L,x0
        move    x0,y:-(r4)
        move    y:(r4)+,x0
        move    x:DS_HOLD_R,x0
        move    x0,y:-(r5)
        move    y:(r5)+,x0
rate_done:
        nop
distortion_loop:
fx_distortion_end:
        rts

; ---- one sample (x0) through the distortion: a = what comes out
distort_one:
        move    x:DS_GAIN,y0
        mpy     y0,x0,a
        asl     #4,a,a
        move    a,x0
        jset    #0,x:FX_SCRATCH,distort_light
        jset    #1,x:FX_SCRATCH,distort_soft
        jset    #2,x:FX_SCRATCH,distort_hard
        jset    #3,x:FX_SCRATCH,distort_bits
        tfr     x0,a                    ; the rate reducer alone: as it is
        rts
distort_light:
        move    x0,a
        asr     a
        move    a,x0
        jsr     soft_clip
        tfr     b,a
        asl     a
        rts
distort_soft:
        jsr     soft_clip
        tfr     b,a
        rts
distort_hard:
        move    x0,a
        asl     #2,a,a
        move    a,x0
        tfr     x0,a
        rts
distort_bits:
        move    x0,a
        move    x:DS_STEP,x1
        and     x1,a
        rts

; ==== chorus or flanger: each side read back from a line, the time moving
; (the other way on the right), fed back for the flanger
fx_chorus:
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
        ; the dry samples into the lines, with what came back
        move    x:CH_WET_L,x0
        move    x:CH_FB,y0
        move    y:(r4),a
        mac     y0,x0,a
        move    a,y:(r6)
        move    x:CH_WET_R,x0
        move    y:(r5),a
        mac     y0,x0,a
        move    a,y:(r7)
        ; left
        move    x:CH_TRI,y1
        move    x:CH_DEPTH,y0
        move    x:CH_BASE,a
        mac     y1,y0,a
        move    r6,r1
        jsr     chorus_read
        move    x0,x:CH_WET_L
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
        move    x0,x:CH_WET_R
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
        rts

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

; ==== phaser: six all-passes a side, their turning point moved by a triangle
; (the other way on the right), fed back
fx_phaser:
        move    x:PH_MIX,a
        tst     a
        jeq     phaser_done
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        move    x:G_N,x0
        do      x0,phaser_end
        move    x:PH_PH,a
        move    x:PH_INC,x0
        add     x0,a
        move    a1,x:PH_PH
        move    a1,x1
        move    x1,a
        abs     a
        asl     a
        neg     a
        move    #>$7fffff,x0
        add     x0,a
        move    a,y1                    ; y1 = the triangle
        ; left
        move    x:PH_DEPTH,y0
        move    x:PH_BASE,a
        mac     y1,y0,a
        move    a,x:PH_COEF
        move    #>PH_STAGES_L,r6
        move    x:PH_LAST_L,x0
        move    x:PH_FB,y0
        move    y:(r4),a
        mac     y0,x0,a
        move    a,x0
        jsr     phaser_stages
        move    x0,x:PH_LAST_L
        move    y:(r4),b
        move    x:PH_MIX,y0
        mac     y0,x0,b
        move    b,y:(r4)+
        ; right
        move    x:PH_PH,x1
        move    x1,a
        abs     a
        asl     a
        neg     a
        move    #>$7fffff,x0
        add     x0,a
        move    a,y1
        move    x:PH_DEPTH,y0
        move    x:PH_BASE,a
        mac     -y1,y0,a
        move    a,x:PH_COEF
        move    #>PH_STAGES_R,r6
        move    x:PH_LAST_R,x0
        move    x:PH_FB,y0
        move    y:(r5),a
        mac     y0,x0,a
        move    a,x0
        jsr     phaser_stages
        move    x0,x:PH_LAST_R
        move    y:(r5),b
        move    x:PH_MIX,y0
        mac     y0,x0,b
        move    b,y:(r5)+
phaser_end:
phaser_done:
        rts

; ---- six first order all-passes (x0 in and out, r6 their state, x:PH_COEF their coefficient):
; y = c x + x' - c y'
phaser_stages:
        move    x:PH_COEF,y0
        do      #6,phaser_stage_end
        mpy     y0,x0,a
        move    x:(r6),x1
        add     x1,a
        move    x0,x:(r6)+
        move    x:(r6),x1
        mac     -x1,y0,a
        move    a,x:(r6)+
        move    a,x0
phaser_stage_end:
        rts

; ==== delay: the left line feeds the right and the right the left, darkened
fx_delay:
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
        rts

; ==== reverb: four combs side by side, two all-passes after them
fx_reverb:
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
        move    x:FX_A,x1
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

; ==== analog boost: the low end, low passed and added again
fx_boost:
        move    x:BO_AMT,a
        tst     a
        jeq     boost_done
        move    #>OUT_L,r4
        move    #>OUT_R,r5
        move    x:G_N,x0
        do      x0,boost_end
        move    y:(r4),x1
        move    x:BO_L,a
        move    x1,b
        sub     a,b
        move    b,x0
        move    x:BO_COEF,y0
        mac     y0,x0,a
        move    a,x:BO_L
        move    a,x0
        move    x:BO_AMT,y0
        tfr     x1,b
        mac     y0,x0,b
        mac     y0,x0,b
        move    b,y:(r4)+
        move    y:(r5),x1
        move    x:BO_R,a
        move    x1,b
        sub     a,b
        move    b,x0
        move    x:BO_COEF,y0
        mac     y0,x0,a
        move    a,x:BO_R
        move    a,x0
        move    x:BO_AMT,y0
        tfr     x1,b
        mac     y0,x0,b
        mac     y0,x0,b
        move    b,y:(r5)+
boost_end:
boost_done:
        rts
