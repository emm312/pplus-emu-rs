#addr 0
; #include "programs/mocker.asm"
; #include "programs/spammer.asm"
	lsi s1 27
	lsi s3 1
	mov rf s3
	loop:
		out s1 0x00
		inc s2
		jmp even s1.evn
		lsl s3 s1
		add s1 s3
		inc s1
		jmp ip loop
		even:
		lsr s1 s1
	jmp loop s1.nrf
	out s2 0