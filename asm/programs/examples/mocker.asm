lsi t0 3
mov rf t0
loop:
	inp t0 0x00
	prd t0.ref
	hlt
	jmp zr t0.zer
	out t0 0x00
jmp zr