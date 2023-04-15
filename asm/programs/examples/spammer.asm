ttypkd = 0x00
outter:
    lfi t5 message
    inner:
        mld t0 t5+
        out t0 ttypkd
        and t0 0x00FF
    jmp ip inner nz
jmp ip outter

message:
#d "Hello!\n"
#align 16
