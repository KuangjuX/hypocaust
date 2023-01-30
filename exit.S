
exit:     file format elf64-littleriscv


Disassembly of section .text:

0000000000010000 <_start>:
   10000:	1101                	addi	sp,sp,-32
   10002:	ec06                	sd	ra,24(sp)
   10004:	e822                	sd	s0,16(sp)
   10006:	e426                	sd	s1,8(sp)
   10008:	e04a                	sd	s2,0(sp)
   1000a:	1000                	addi	s0,sp,32

000000000001000c <.LBB0_3>:
   1000c:	00007517          	auipc	a0,0x7
   10010:	34c50513          	addi	a0,a0,844 # 17358 <_ZN8user_lib4HEAP17he3af741f2715d7f2E>
   10014:	00001097          	auipc	ra,0x1
   10018:	856080e7          	jalr	-1962(ra) # 1086a <_ZN78_$LT$buddy_system_allocator..LockedHeap$u20$as$u20$core..ops..deref..Deref$GT$5deref17h009de595a571d1f5E>
   1001c:	892a                	mv	s2,a0
   1001e:	4505                	li	a0,1
   10020:	00a934af          	amoadd.d	s1,a0,(s2)
   10024:	00893503          	ld	a0,8(s2)
   10028:	0230000f          	fence	r,rw
   1002c:	00950a63          	beq	a0,s1,10040 <.LBB0_3+0x34>
   10030:	0100000f          	fence	w,unknown
   10034:	00893503          	ld	a0,8(s2)
   10038:	0230000f          	fence	r,rw
   1003c:	fe951ae3          	bne	a0,s1,10030 <.LBB0_3+0x24>
   10040:	01090513          	addi	a0,s2,16

0000000000010044 <.LBB0_4>:
   10044:	00003597          	auipc	a1,0x3
   10048:	31458593          	addi	a1,a1,788 # 13358 <_ZN8user_lib10HEAP_SPACE17h53125ea949b2d318E>
   1004c:	6611                	lui	a2,0x4
   1004e:	00000097          	auipc	ra,0x0
   10052:	6b8080e7          	jalr	1720(ra) # 10706 <_ZN22buddy_system_allocator4Heap4init17h1d037308586cbd83E>
   10056:	00148513          	addi	a0,s1,1
   1005a:	0310000f          	fence	rw,w
   1005e:	00a93423          	sd	a0,8(s2)
   10062:	00000097          	auipc	ra,0x0
   10066:	012080e7          	jalr	18(ra) # 10074 <main>
   1006a:	00000097          	auipc	ra,0x0
   1006e:	3f8080e7          	jalr	1016(ra) # 10462 <_ZN8user_lib4exit17hd894eb19750960c3E>
	...

0000000000010074 <main>:
   10074:	7119                	addi	sp,sp,-128
   10076:	fc86                	sd	ra,120(sp)
   10078:	f8a2                	sd	s0,112(sp)
   1007a:	f4a6                	sd	s1,104(sp)
   1007c:	f0ca                	sd	s2,96(sp)
   1007e:	ecce                	sd	s3,88(sp)
   10080:	0100                	addi	s0,sp,128

0000000000010082 <.LBB0_8>:
   10082:	00002517          	auipc	a0,0x2
   10086:	fa650513          	addi	a0,a0,-90 # 12028 <.Lanon.fad58de7366495db4650cfefac2fcd61.2>
   1008a:	faa43c23          	sd	a0,-72(s0)
   1008e:	4985                	li	s3,1
   10090:	fd343023          	sd	s3,-64(s0)
   10094:	fa043423          	sd	zero,-88(s0)

0000000000010098 <.LBB0_9>:
   10098:	00002917          	auipc	s2,0x2
   1009c:	f6890913          	addi	s2,s2,-152 # 12000 <.Lanon.fad58de7366495db4650cfefac2fcd61.1>
   100a0:	fd243423          	sd	s2,-56(s0)
   100a4:	fc043823          	sd	zero,-48(s0)
   100a8:	fa840513          	addi	a0,s0,-88
   100ac:	00000097          	auipc	ra,0x0
   100b0:	336080e7          	jalr	822(ra) # 103e2 <_ZN8user_lib7console5print17hffb6612453cd7deaE>
   100b4:	00000097          	auipc	ra,0x0
   100b8:	3de080e7          	jalr	990(ra) # 10492 <_ZN8user_lib4fork17hcc4d707b6aac339eE>
   100bc:	f8a43423          	sd	a0,-120(s0)
   100c0:	14050663          	beqz	a0,1020c <.LBB0_17>
   100c4:	f8840513          	addi	a0,s0,-120
   100c8:	f8a43c23          	sd	a0,-104(s0)

00000000000100cc <.LBB0_10>:
   100cc:	00001497          	auipc	s1,0x1
   100d0:	26048493          	addi	s1,s1,608 # 1132c <_ZN4core3fmt3num3imp52_$LT$impl$u20$core..fmt..Display$u20$for$u20$i64$GT$3fmt17h3dda09ee44bad10dE>
   100d4:	fa943023          	sd	s1,-96(s0)

00000000000100d8 <.LBB0_11>:
   100d8:	00002517          	auipc	a0,0x2
   100dc:	f8050513          	addi	a0,a0,-128 # 12058 <.Lanon.fad58de7366495db4650cfefac2fcd61.5>
   100e0:	faa43c23          	sd	a0,-72(s0)
   100e4:	4509                	li	a0,2
   100e6:	fca43023          	sd	a0,-64(s0)
   100ea:	fa043423          	sd	zero,-88(s0)
   100ee:	f9840513          	addi	a0,s0,-104
   100f2:	fca43423          	sd	a0,-56(s0)
   100f6:	fd343823          	sd	s3,-48(s0)
   100fa:	fa840513          	addi	a0,s0,-88
   100fe:	00000097          	auipc	ra,0x0
   10102:	2e4080e7          	jalr	740(ra) # 103e2 <_ZN8user_lib7console5print17hffb6612453cd7deaE>

0000000000010106 <.LBB0_12>:
   10106:	00002517          	auipc	a0,0x2
   1010a:	f9250513          	addi	a0,a0,-110 # 12098 <.Lanon.fad58de7366495db4650cfefac2fcd61.7>
   1010e:	faa43c23          	sd	a0,-72(s0)
   10112:	fd343023          	sd	s3,-64(s0)
   10116:	fa043423          	sd	zero,-88(s0)
   1011a:	fd243423          	sd	s2,-56(s0)
   1011e:	fc043823          	sd	zero,-48(s0)
   10122:	fa840513          	addi	a0,s0,-88
   10126:	00000097          	auipc	ra,0x0
   1012a:	2bc080e7          	jalr	700(ra) # 103e2 <_ZN8user_lib7console5print17hffb6612453cd7deaE>
   1012e:	f8843503          	ld	a0,-120(s0)
   10132:	f8042a23          	sw	zero,-108(s0)
   10136:	f9440593          	addi	a1,s0,-108
   1013a:	00000097          	auipc	ra,0x0
   1013e:	3bc080e7          	jalr	956(ra) # 104f6 <_ZN8user_lib7waitpid17h6f6b0e166f1f3e3bE>
   10142:	f8843583          	ld	a1,-120(s0)
   10146:	f9442603          	lw	a2,-108(s0)
   1014a:	8da9                	xor	a1,a1,a0
   1014c:	76c1                	lui	a3,0xffff0
   1014e:	c7c6869b          	addiw	a3,a3,-900
   10152:	8e35                	xor	a2,a2,a3
   10154:	8dd1                	or	a1,a1,a2
   10156:	12059363          	bnez	a1,1027c <.LBB0_18>
   1015a:	f9440593          	addi	a1,s0,-108
   1015e:	00000097          	auipc	ra,0x0
   10162:	398080e7          	jalr	920(ra) # 104f6 <_ZN8user_lib7waitpid17h6f6b0e166f1f3e3bE>
   10166:	08055463          	bgez	a0,101ee <.LBB0_15>
   1016a:	f9440513          	addi	a0,s0,-108
   1016e:	00000097          	auipc	ra,0x0
   10172:	342080e7          	jalr	834(ra) # 104b0 <_ZN8user_lib4wait17h76c070cdf177886cE>
   10176:	06a04c63          	bgtz	a0,101ee <.LBB0_15>
   1017a:	f8840513          	addi	a0,s0,-120
   1017e:	f8a43c23          	sd	a0,-104(s0)
   10182:	fa943023          	sd	s1,-96(s0)

0000000000010186 <.LBB0_13>:
   10186:	00002517          	auipc	a0,0x2
   1018a:	01a50513          	addi	a0,a0,26 # 121a0 <.Lanon.fad58de7366495db4650cfefac2fcd61.15>
   1018e:	faa43c23          	sd	a0,-72(s0)
   10192:	4509                	li	a0,2
   10194:	fca43023          	sd	a0,-64(s0)
   10198:	fa043423          	sd	zero,-88(s0)
   1019c:	f9840513          	addi	a0,s0,-104
   101a0:	fca43423          	sd	a0,-56(s0)
   101a4:	4485                	li	s1,1
   101a6:	fc943823          	sd	s1,-48(s0)
   101aa:	fa840513          	addi	a0,s0,-88
   101ae:	00000097          	auipc	ra,0x0
   101b2:	234080e7          	jalr	564(ra) # 103e2 <_ZN8user_lib7console5print17hffb6612453cd7deaE>

00000000000101b6 <.LBB0_14>:
   101b6:	00002517          	auipc	a0,0x2
   101ba:	01a50513          	addi	a0,a0,26 # 121d0 <.Lanon.fad58de7366495db4650cfefac2fcd61.17>
   101be:	faa43c23          	sd	a0,-72(s0)
   101c2:	fc943023          	sd	s1,-64(s0)
   101c6:	fa043423          	sd	zero,-88(s0)
   101ca:	fd243423          	sd	s2,-56(s0)
   101ce:	fc043823          	sd	zero,-48(s0)
   101d2:	fa840513          	addi	a0,s0,-88
   101d6:	00000097          	auipc	ra,0x0
   101da:	20c080e7          	jalr	524(ra) # 103e2 <_ZN8user_lib7console5print17hffb6612453cd7deaE>
   101de:	4501                	li	a0,0
   101e0:	70e6                	ld	ra,120(sp)
   101e2:	7446                	ld	s0,112(sp)
   101e4:	74a6                	ld	s1,104(sp)
   101e6:	7906                	ld	s2,96(sp)
   101e8:	69e6                	ld	s3,88(sp)
   101ea:	6109                	addi	sp,sp,128
   101ec:	8082                	ret

00000000000101ee <.LBB0_15>:
   101ee:	00002517          	auipc	a0,0x2
   101f2:	f3250513          	addi	a0,a0,-206 # 12120 <.Lanon.fad58de7366495db4650cfefac2fcd61.11>

00000000000101f6 <.LBB0_16>:
   101f6:	00002617          	auipc	a2,0x2
   101fa:	f8260613          	addi	a2,a2,-126 # 12178 <.Lanon.fad58de7366495db4650cfefac2fcd61.12>
   101fe:	05200593          	li	a1,82
   10202:	00000097          	auipc	ra,0x0
   10206:	6c0080e7          	jalr	1728(ra) # 108c2 <_ZN4core9panicking5panic17h0ed902ae04a4e669E>
	...

000000000001020c <.LBB0_17>:
   1020c:	00002517          	auipc	a0,0x2
   10210:	fe450513          	addi	a0,a0,-28 # 121f0 <.Lanon.fad58de7366495db4650cfefac2fcd61.19>
   10214:	faa43c23          	sd	a0,-72(s0)
   10218:	fd343023          	sd	s3,-64(s0)
   1021c:	fa043423          	sd	zero,-88(s0)
   10220:	fd243423          	sd	s2,-56(s0)
   10224:	fc043823          	sd	zero,-48(s0)
   10228:	fa840513          	addi	a0,s0,-88
   1022c:	00000097          	auipc	ra,0x0
   10230:	1b6080e7          	jalr	438(ra) # 103e2 <_ZN8user_lib7console5print17hffb6612453cd7deaE>
   10234:	00000097          	auipc	ra,0x0
   10238:	240080e7          	jalr	576(ra) # 10474 <_ZN8user_lib6yield_17h5119a6b64bb7a0f7E>
   1023c:	00000097          	auipc	ra,0x0
   10240:	238080e7          	jalr	568(ra) # 10474 <_ZN8user_lib6yield_17h5119a6b64bb7a0f7E>
   10244:	00000097          	auipc	ra,0x0
   10248:	230080e7          	jalr	560(ra) # 10474 <_ZN8user_lib6yield_17h5119a6b64bb7a0f7E>
   1024c:	00000097          	auipc	ra,0x0
   10250:	228080e7          	jalr	552(ra) # 10474 <_ZN8user_lib6yield_17h5119a6b64bb7a0f7E>
   10254:	00000097          	auipc	ra,0x0
   10258:	220080e7          	jalr	544(ra) # 10474 <_ZN8user_lib6yield_17h5119a6b64bb7a0f7E>
   1025c:	00000097          	auipc	ra,0x0
   10260:	218080e7          	jalr	536(ra) # 10474 <_ZN8user_lib6yield_17h5119a6b64bb7a0f7E>
   10264:	00000097          	auipc	ra,0x0
   10268:	210080e7          	jalr	528(ra) # 10474 <_ZN8user_lib6yield_17h5119a6b64bb7a0f7E>
   1026c:	7541                	lui	a0,0xffff0
   1026e:	c7c5051b          	addiw	a0,a0,-900
   10272:	00000097          	auipc	ra,0x0
   10276:	1f0080e7          	jalr	496(ra) # 10462 <_ZN8user_lib4exit17hd894eb19750960c3E>
	...

000000000001027c <.LBB0_18>:
   1027c:	00002517          	auipc	a0,0x2
   10280:	e2c50513          	addi	a0,a0,-468 # 120a8 <.Lanon.fad58de7366495db4650cfefac2fcd61.8>

0000000000010284 <.LBB0_19>:
   10284:	00002617          	auipc	a2,0x2
   10288:	e8460613          	addi	a2,a2,-380 # 12108 <.Lanon.fad58de7366495db4650cfefac2fcd61.10>
   1028c:	04e00593          	li	a1,78
   10290:	00000097          	auipc	ra,0x0
   10294:	632080e7          	jalr	1586(ra) # 108c2 <_ZN4core9panicking5panic17h0ed902ae04a4e669E>
	...

000000000001029a <_ZN4core3ptr37drop_in_place$LT$core..fmt..Error$GT$17h2c709566e0a07aeeE.llvm.15042265727004590274>:
   1029a:	1141                	addi	sp,sp,-16
   1029c:	e406                	sd	ra,8(sp)
   1029e:	e022                	sd	s0,0(sp)
   102a0:	0800                	addi	s0,sp,16
   102a2:	60a2                	ld	ra,8(sp)
   102a4:	6402                	ld	s0,0(sp)
   102a6:	0141                	addi	sp,sp,16
   102a8:	8082                	ret

00000000000102aa <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17hee404a9537250452E.llvm.15042265727004590274>:
   102aa:	1101                	addi	sp,sp,-32
   102ac:	ec06                	sd	ra,24(sp)
   102ae:	e822                	sd	s0,16(sp)
   102b0:	1000                	addi	s0,sp,32
   102b2:	0005851b          	sext.w	a0,a1
   102b6:	08000613          	li	a2,128
   102ba:	fe042623          	sw	zero,-20(s0)
   102be:	00c57663          	bgeu	a0,a2,102ca <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17hee404a9537250452E.llvm.15042265727004590274+0x20>
   102c2:	feb40623          	sb	a1,-20(s0)
   102c6:	4605                	li	a2,1
   102c8:	a851                	j	1035c <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17hee404a9537250452E.llvm.15042265727004590274+0xb2>
   102ca:	00b5d51b          	srliw	a0,a1,0xb
   102ce:	ed19                	bnez	a0,102ec <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17hee404a9537250452E.llvm.15042265727004590274+0x42>
   102d0:	0065d513          	srli	a0,a1,0x6
   102d4:	0c056513          	ori	a0,a0,192
   102d8:	fea40623          	sb	a0,-20(s0)
   102dc:	03f5f513          	andi	a0,a1,63
   102e0:	08056513          	ori	a0,a0,128
   102e4:	fea406a3          	sb	a0,-19(s0)
   102e8:	4609                	li	a2,2
   102ea:	a88d                	j	1035c <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17hee404a9537250452E.llvm.15042265727004590274+0xb2>
   102ec:	0105d51b          	srliw	a0,a1,0x10
   102f0:	e905                	bnez	a0,10320 <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17hee404a9537250452E.llvm.15042265727004590274+0x76>
   102f2:	02059513          	slli	a0,a1,0x20
   102f6:	9101                	srli	a0,a0,0x20
   102f8:	00c5d61b          	srliw	a2,a1,0xc
   102fc:	0e066613          	ori	a2,a2,224
   10300:	fec40623          	sb	a2,-20(s0)
   10304:	1552                	slli	a0,a0,0x34
   10306:	9169                	srli	a0,a0,0x3a
   10308:	08056513          	ori	a0,a0,128
   1030c:	fea406a3          	sb	a0,-19(s0)
   10310:	03f5f513          	andi	a0,a1,63
   10314:	08056513          	ori	a0,a0,128
   10318:	fea40723          	sb	a0,-18(s0)
   1031c:	460d                	li	a2,3
   1031e:	a83d                	j	1035c <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$10write_char17hee404a9537250452E.llvm.15042265727004590274+0xb2>
   10320:	02059513          	slli	a0,a1,0x20
   10324:	9101                	srli	a0,a0,0x20
   10326:	02b51613          	slli	a2,a0,0x2b
   1032a:	9275                	srli	a2,a2,0x3d
   1032c:	0f066613          	ori	a2,a2,240
   10330:	fec40623          	sb	a2,-20(s0)
   10334:	02e51613          	slli	a2,a0,0x2e
   10338:	9269                	srli	a2,a2,0x3a
   1033a:	08066613          	ori	a2,a2,128
   1033e:	fec406a3          	sb	a2,-19(s0)
   10342:	1552                	slli	a0,a0,0x34
   10344:	9169                	srli	a0,a0,0x3a
   10346:	08056513          	ori	a0,a0,128
   1034a:	fea40723          	sb	a0,-18(s0)
   1034e:	03f5f513          	andi	a0,a1,63
   10352:	08056513          	ori	a0,a0,128
   10356:	fea407a3          	sb	a0,-17(s0)
   1035a:	4611                	li	a2,4
   1035c:	4505                	li	a0,1
   1035e:	fec40593          	addi	a1,s0,-20
   10362:	04000893          	li	a7,64
   10366:	00000073          	ecall
   1036a:	4501                	li	a0,0
   1036c:	60e2                	ld	ra,24(sp)
   1036e:	6442                	ld	s0,16(sp)
   10370:	6105                	addi	sp,sp,32
   10372:	8082                	ret

0000000000010374 <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$9write_fmt17h7e542ec7faad1cccE.llvm.15042265727004590274>:
   10374:	715d                	addi	sp,sp,-80
   10376:	e486                	sd	ra,72(sp)
   10378:	e0a2                	sd	s0,64(sp)
   1037a:	0880                	addi	s0,sp,80
   1037c:	6108                	ld	a0,0(a0)
   1037e:	7590                	ld	a2,40(a1)
   10380:	7194                	ld	a3,32(a1)
   10382:	faa43c23          	sd	a0,-72(s0)
   10386:	fec43423          	sd	a2,-24(s0)
   1038a:	fed43023          	sd	a3,-32(s0)
   1038e:	6d88                	ld	a0,24(a1)
   10390:	6990                	ld	a2,16(a1)
   10392:	6594                	ld	a3,8(a1)
   10394:	618c                	ld	a1,0(a1)
   10396:	fca43c23          	sd	a0,-40(s0)
   1039a:	fcc43823          	sd	a2,-48(s0)
   1039e:	fcd43423          	sd	a3,-56(s0)
   103a2:	fcb43023          	sd	a1,-64(s0)

00000000000103a6 <.LBB2_1>:
   103a6:	00002597          	auipc	a1,0x2
   103aa:	e5a58593          	addi	a1,a1,-422 # 12200 <anon.25d4c90b4a8e3929f596ac6ff3381d35.0.llvm.15042265727004590274>
   103ae:	fb840513          	addi	a0,s0,-72
   103b2:	fc040613          	addi	a2,s0,-64
   103b6:	00000097          	auipc	ra,0x0
   103ba:	5f6080e7          	jalr	1526(ra) # 109ac <_ZN4core3fmt5write17h9d53518631e11158E>
   103be:	60a6                	ld	ra,72(sp)
   103c0:	6406                	ld	s0,64(sp)
   103c2:	6161                	addi	sp,sp,80
   103c4:	8082                	ret

00000000000103c6 <_ZN50_$LT$$RF$mut$u20$W$u20$as$u20$core..fmt..Write$GT$9write_str17hb37d46d5a515b22fE.llvm.15042265727004590274>:
   103c6:	1141                	addi	sp,sp,-16
   103c8:	e406                	sd	ra,8(sp)
   103ca:	e022                	sd	s0,0(sp)
   103cc:	0800                	addi	s0,sp,16
   103ce:	4505                	li	a0,1
   103d0:	04000893          	li	a7,64
   103d4:	00000073          	ecall
   103d8:	4501                	li	a0,0
   103da:	60a2                	ld	ra,8(sp)
   103dc:	6402                	ld	s0,0(sp)
   103de:	0141                	addi	sp,sp,16
   103e0:	8082                	ret

00000000000103e2 <_ZN8user_lib7console5print17hffb6612453cd7deaE>:
   103e2:	715d                	addi	sp,sp,-80
   103e4:	e486                	sd	ra,72(sp)
   103e6:	e0a2                	sd	s0,64(sp)
   103e8:	0880                	addi	s0,sp,80
   103ea:	750c                	ld	a1,40(a0)
   103ec:	7110                	ld	a2,32(a0)
   103ee:	fe840693          	addi	a3,s0,-24
   103f2:	fad43823          	sd	a3,-80(s0)
   103f6:	feb43023          	sd	a1,-32(s0)
   103fa:	fcc43c23          	sd	a2,-40(s0)
   103fe:	6d0c                	ld	a1,24(a0)
   10400:	6910                	ld	a2,16(a0)
   10402:	6514                	ld	a3,8(a0)
   10404:	6108                	ld	a0,0(a0)
   10406:	fcb43823          	sd	a1,-48(s0)
   1040a:	fcc43423          	sd	a2,-56(s0)
   1040e:	fcd43023          	sd	a3,-64(s0)
   10412:	faa43c23          	sd	a0,-72(s0)

0000000000010416 <.LBB5_3>:
   10416:	00002597          	auipc	a1,0x2
   1041a:	dea58593          	addi	a1,a1,-534 # 12200 <anon.25d4c90b4a8e3929f596ac6ff3381d35.0.llvm.15042265727004590274>
   1041e:	fb040513          	addi	a0,s0,-80
   10422:	fb840613          	addi	a2,s0,-72
   10426:	00000097          	auipc	ra,0x0
   1042a:	586080e7          	jalr	1414(ra) # 109ac <_ZN4core3fmt5write17h9d53518631e11158E>
   1042e:	e509                	bnez	a0,10438 <.LBB5_4>
   10430:	60a6                	ld	ra,72(sp)
   10432:	6406                	ld	s0,64(sp)
   10434:	6161                	addi	sp,sp,80
   10436:	8082                	ret

0000000000010438 <.LBB5_4>:
   10438:	00002517          	auipc	a0,0x2
   1043c:	df850513          	addi	a0,a0,-520 # 12230 <anon.25d4c90b4a8e3929f596ac6ff3381d35.1.llvm.15042265727004590274>

0000000000010440 <.LBB5_5>:
   10440:	00002697          	auipc	a3,0x2
   10444:	e2068693          	addi	a3,a3,-480 # 12260 <anon.25d4c90b4a8e3929f596ac6ff3381d35.2.llvm.15042265727004590274>

0000000000010448 <.LBB5_6>:
   10448:	00002717          	auipc	a4,0x2
   1044c:	e4870713          	addi	a4,a4,-440 # 12290 <anon.25d4c90b4a8e3929f596ac6ff3381d35.4.llvm.15042265727004590274>
   10450:	02b00593          	li	a1,43
   10454:	fe840613          	addi	a2,s0,-24
   10458:	00000097          	auipc	ra,0x0
   1045c:	4d6080e7          	jalr	1238(ra) # 1092e <_ZN4core6result13unwrap_failed17h1ccf3470a5333e86E>
	...

0000000000010462 <_ZN8user_lib4exit17hd894eb19750960c3E>:
   10462:	1141                	addi	sp,sp,-16
   10464:	e406                	sd	ra,8(sp)
   10466:	e022                	sd	s0,0(sp)
   10468:	0800                	addi	s0,sp,16
   1046a:	00000097          	auipc	ra,0x0
   1046e:	0d0080e7          	jalr	208(ra) # 1053a <_ZN8user_lib7syscall8sys_exit17hd69fd813f3352a49E>
	...

0000000000010474 <_ZN8user_lib6yield_17h5119a6b64bb7a0f7E>:
   10474:	1141                	addi	sp,sp,-16
   10476:	e406                	sd	ra,8(sp)
   10478:	e022                	sd	s0,0(sp)
   1047a:	0800                	addi	s0,sp,16
   1047c:	07c00893          	li	a7,124
   10480:	4501                	li	a0,0
   10482:	4581                	li	a1,0
   10484:	4601                	li	a2,0
   10486:	00000073          	ecall
   1048a:	60a2                	ld	ra,8(sp)
   1048c:	6402                	ld	s0,0(sp)
   1048e:	0141                	addi	sp,sp,16
   10490:	8082                	ret

0000000000010492 <_ZN8user_lib4fork17hcc4d707b6aac339eE>:
   10492:	1141                	addi	sp,sp,-16
   10494:	e406                	sd	ra,8(sp)
   10496:	e022                	sd	s0,0(sp)
   10498:	0800                	addi	s0,sp,16
   1049a:	0dc00893          	li	a7,220
   1049e:	4501                	li	a0,0
   104a0:	4581                	li	a1,0
   104a2:	4601                	li	a2,0
   104a4:	00000073          	ecall
   104a8:	60a2                	ld	ra,8(sp)
   104aa:	6402                	ld	s0,0(sp)
   104ac:	0141                	addi	sp,sp,16
   104ae:	8082                	ret

00000000000104b0 <_ZN8user_lib4wait17h76c070cdf177886cE>:
   104b0:	1141                	addi	sp,sp,-16
   104b2:	e406                	sd	ra,8(sp)
   104b4:	e022                	sd	s0,0(sp)
   104b6:	0800                	addi	s0,sp,16
   104b8:	86aa                	mv	a3,a0
   104ba:	557d                	li	a0,-1
   104bc:	10400893          	li	a7,260
   104c0:	5779                	li	a4,-2
   104c2:	85b6                	mv	a1,a3
   104c4:	4601                	li	a2,0
   104c6:	00000073          	ecall
   104ca:	02e51263          	bne	a0,a4,104ee <_ZN8user_lib4wait17h76c070cdf177886cE+0x3e>
   104ce:	07c00893          	li	a7,124
   104d2:	4501                	li	a0,0
   104d4:	4581                	li	a1,0
   104d6:	4601                	li	a2,0
   104d8:	00000073          	ecall
   104dc:	557d                	li	a0,-1
   104de:	10400893          	li	a7,260
   104e2:	85b6                	mv	a1,a3
   104e4:	4601                	li	a2,0
   104e6:	00000073          	ecall
   104ea:	fee502e3          	beq	a0,a4,104ce <_ZN8user_lib4wait17h76c070cdf177886cE+0x1e>
   104ee:	60a2                	ld	ra,8(sp)
   104f0:	6402                	ld	s0,0(sp)
   104f2:	0141                	addi	sp,sp,16
   104f4:	8082                	ret

00000000000104f6 <_ZN8user_lib7waitpid17h6f6b0e166f1f3e3bE>:
   104f6:	1141                	addi	sp,sp,-16
   104f8:	e406                	sd	ra,8(sp)
   104fa:	e022                	sd	s0,0(sp)
   104fc:	0800                	addi	s0,sp,16
   104fe:	872a                	mv	a4,a0
   10500:	10400893          	li	a7,260
   10504:	57f9                	li	a5,-2
   10506:	4601                	li	a2,0
   10508:	00000073          	ecall
   1050c:	02f51363          	bne	a0,a5,10532 <_ZN8user_lib7waitpid17h6f6b0e166f1f3e3bE+0x3c>
   10510:	86ae                	mv	a3,a1
   10512:	07c00893          	li	a7,124
   10516:	4501                	li	a0,0
   10518:	4581                	li	a1,0
   1051a:	4601                	li	a2,0
   1051c:	00000073          	ecall
   10520:	10400893          	li	a7,260
   10524:	853a                	mv	a0,a4
   10526:	85b6                	mv	a1,a3
   10528:	4601                	li	a2,0
   1052a:	00000073          	ecall
   1052e:	fef502e3          	beq	a0,a5,10512 <_ZN8user_lib7waitpid17h6f6b0e166f1f3e3bE+0x1c>
   10532:	60a2                	ld	ra,8(sp)
   10534:	6402                	ld	s0,0(sp)
   10536:	0141                	addi	sp,sp,16
   10538:	8082                	ret

000000000001053a <_ZN8user_lib7syscall8sys_exit17hd69fd813f3352a49E>:
   1053a:	1141                	addi	sp,sp,-16
   1053c:	e406                	sd	ra,8(sp)
   1053e:	e022                	sd	s0,0(sp)
   10540:	0800                	addi	s0,sp,16
   10542:	2501                	sext.w	a0,a0
   10544:	05d00893          	li	a7,93
   10548:	4581                	li	a1,0
   1054a:	4601                	li	a2,0
   1054c:	00000073          	ecall

0000000000010550 <.LBB2_1>:
   10550:	00002517          	auipc	a0,0x2
   10554:	d5850513          	addi	a0,a0,-680 # 122a8 <anon.4b2c2c7fda0f5965308fd32871feba1e.0.llvm.4426990138377360633>

0000000000010558 <.LBB2_2>:
   10558:	00002617          	auipc	a2,0x2
   1055c:	d7860613          	addi	a2,a2,-648 # 122d0 <anon.4b2c2c7fda0f5965308fd32871feba1e.2.llvm.4426990138377360633>
   10560:	45dd                	li	a1,23
   10562:	00000097          	auipc	ra,0x0
   10566:	360080e7          	jalr	864(ra) # 108c2 <_ZN4core9panicking5panic17h0ed902ae04a4e669E>
	...

000000000001056c <rust_begin_unwind>:
   1056c:	7171                	addi	sp,sp,-176
   1056e:	f506                	sd	ra,168(sp)
   10570:	f122                	sd	s0,160(sp)
   10572:	ed26                	sd	s1,152(sp)
   10574:	1900                	addi	s0,sp,176
   10576:	84aa                	mv	s1,a0
   10578:	00000097          	auipc	ra,0x0
   1057c:	314080e7          	jalr	788(ra) # 1088c <_ZN4core5panic10panic_info9PanicInfo7message17h8ed89e8633dfae6dE>
   10580:	12050763          	beqz	a0,106ae <.LBB0_18>
   10584:	f4a43c23          	sd	a0,-168(s0)
   10588:	8526                	mv	a0,s1
   1058a:	00000097          	auipc	ra,0x0
   1058e:	306080e7          	jalr	774(ra) # 10890 <_ZN4core5panic10panic_info9PanicInfo8location17he3738231e6489566E>
   10592:	cd41                	beqz	a0,1062a <.LBB0_11+0x1c>
   10594:	610c                	ld	a1,0(a0)
   10596:	6510                	ld	a2,8(a0)
   10598:	f8b43823          	sd	a1,-112(s0)
   1059c:	f8c43c23          	sd	a2,-104(s0)
   105a0:	4908                	lw	a0,16(a0)
   105a2:	faa42223          	sw	a0,-92(s0)
   105a6:	f9040513          	addi	a0,s0,-112
   105aa:	f6a43023          	sd	a0,-160(s0)

00000000000105ae <.LBB0_7>:
   105ae:	00000517          	auipc	a0,0x0
   105b2:	11e50513          	addi	a0,a0,286 # 106cc <_ZN44_$LT$$RF$T$u20$as$u20$core..fmt..Display$GT$3fmt17h7325186bec23cf13E>
   105b6:	f6a43423          	sd	a0,-152(s0)
   105ba:	fa440513          	addi	a0,s0,-92
   105be:	f6a43823          	sd	a0,-144(s0)

00000000000105c2 <.LBB0_8>:
   105c2:	00001517          	auipc	a0,0x1
   105c6:	d5a50513          	addi	a0,a0,-678 # 1131c <_ZN4core3fmt3num3imp52_$LT$impl$u20$core..fmt..Display$u20$for$u20$u32$GT$3fmt17h1b2ac9295173ee49E>
   105ca:	f6a43c23          	sd	a0,-136(s0)
   105ce:	f5840513          	addi	a0,s0,-168
   105d2:	f8a43023          	sd	a0,-128(s0)

00000000000105d6 <.LBB0_9>:
   105d6:	00000517          	auipc	a0,0x0
   105da:	11850513          	addi	a0,a0,280 # 106ee <_ZN44_$LT$$RF$T$u20$as$u20$core..fmt..Display$GT$3fmt17hc095759edf7bce08E>
   105de:	f8a43423          	sd	a0,-120(s0)
   105e2:	fe040513          	addi	a0,s0,-32
   105e6:	faa43423          	sd	a0,-88(s0)
   105ea:	fa043823          	sd	zero,-80(s0)

00000000000105ee <.LBB0_10>:
   105ee:	00002517          	auipc	a0,0x2
   105f2:	d3a50513          	addi	a0,a0,-710 # 12328 <.Lanon.86a3613c128665d32fc75176e6ae67c2.11>
   105f6:	fca43023          	sd	a0,-64(s0)
   105fa:	4511                	li	a0,4
   105fc:	fca43423          	sd	a0,-56(s0)
   10600:	f6040513          	addi	a0,s0,-160
   10604:	fca43823          	sd	a0,-48(s0)
   10608:	450d                	li	a0,3
   1060a:	fca43c23          	sd	a0,-40(s0)

000000000001060e <.LBB0_11>:
   1060e:	00002597          	auipc	a1,0x2
   10612:	bf258593          	addi	a1,a1,-1038 # 12200 <anon.25d4c90b4a8e3929f596ac6ff3381d35.0.llvm.15042265727004590274>
   10616:	fa840513          	addi	a0,s0,-88
   1061a:	fb040613          	addi	a2,s0,-80
   1061e:	00000097          	auipc	ra,0x0
   10622:	38e080e7          	jalr	910(ra) # 109ac <_ZN4core3fmt5write17h9d53518631e11158E>
   10626:	ed39                	bnez	a0,10684 <.LBB0_15>
   10628:	a001                	j	10628 <.LBB0_11+0x1a>
   1062a:	f5840513          	addi	a0,s0,-168
   1062e:	f6a43023          	sd	a0,-160(s0)

0000000000010632 <.LBB0_12>:
   10632:	00000517          	auipc	a0,0x0
   10636:	0bc50513          	addi	a0,a0,188 # 106ee <_ZN44_$LT$$RF$T$u20$as$u20$core..fmt..Display$GT$3fmt17hc095759edf7bce08E>
   1063a:	f6a43423          	sd	a0,-152(s0)
   1063e:	fe040513          	addi	a0,s0,-32
   10642:	f8a43823          	sd	a0,-112(s0)
   10646:	fa043823          	sd	zero,-80(s0)

000000000001064a <.LBB0_13>:
   1064a:	00002517          	auipc	a0,0x2
   1064e:	cae50513          	addi	a0,a0,-850 # 122f8 <.Lanon.86a3613c128665d32fc75176e6ae67c2.7>
   10652:	fca43023          	sd	a0,-64(s0)
   10656:	4509                	li	a0,2
   10658:	fca43423          	sd	a0,-56(s0)
   1065c:	f6040513          	addi	a0,s0,-160
   10660:	fca43823          	sd	a0,-48(s0)
   10664:	4505                	li	a0,1
   10666:	fca43c23          	sd	a0,-40(s0)

000000000001066a <.LBB0_14>:
   1066a:	00002597          	auipc	a1,0x2
   1066e:	b9658593          	addi	a1,a1,-1130 # 12200 <anon.25d4c90b4a8e3929f596ac6ff3381d35.0.llvm.15042265727004590274>
   10672:	f9040513          	addi	a0,s0,-112
   10676:	fb040613          	addi	a2,s0,-80
   1067a:	00000097          	auipc	ra,0x0
   1067e:	332080e7          	jalr	818(ra) # 109ac <_ZN4core3fmt5write17h9d53518631e11158E>
   10682:	d15d                	beqz	a0,10628 <.LBB0_11+0x1a>

0000000000010684 <.LBB0_15>:
   10684:	00002517          	auipc	a0,0x2
   10688:	bac50513          	addi	a0,a0,-1108 # 12230 <anon.25d4c90b4a8e3929f596ac6ff3381d35.1.llvm.15042265727004590274>

000000000001068c <.LBB0_16>:
   1068c:	00002697          	auipc	a3,0x2
   10690:	bd468693          	addi	a3,a3,-1068 # 12260 <anon.25d4c90b4a8e3929f596ac6ff3381d35.2.llvm.15042265727004590274>

0000000000010694 <.LBB0_17>:
   10694:	00002717          	auipc	a4,0x2
   10698:	bfc70713          	addi	a4,a4,-1028 # 12290 <anon.25d4c90b4a8e3929f596ac6ff3381d35.4.llvm.15042265727004590274>
   1069c:	02b00593          	li	a1,43
   106a0:	fe040613          	addi	a2,s0,-32
   106a4:	00000097          	auipc	ra,0x0
   106a8:	28a080e7          	jalr	650(ra) # 1092e <_ZN4core6result13unwrap_failed17h1ccf3470a5333e86E>
	...

00000000000106ae <.LBB0_18>:
   106ae:	00002517          	auipc	a0,0x2
   106b2:	cba50513          	addi	a0,a0,-838 # 12368 <.Lanon.86a3613c128665d32fc75176e6ae67c2.12>

00000000000106b6 <.LBB0_19>:
   106b6:	00002617          	auipc	a2,0x2
   106ba:	cf260613          	addi	a2,a2,-782 # 123a8 <.Lanon.86a3613c128665d32fc75176e6ae67c2.14>
   106be:	02b00593          	li	a1,43
   106c2:	00000097          	auipc	ra,0x0
   106c6:	200080e7          	jalr	512(ra) # 108c2 <_ZN4core9panicking5panic17h0ed902ae04a4e669E>
	...

00000000000106cc <_ZN44_$LT$$RF$T$u20$as$u20$core..fmt..Display$GT$3fmt17h7325186bec23cf13E>:
   106cc:	1141                	addi	sp,sp,-16
   106ce:	e406                	sd	ra,8(sp)
   106d0:	e022                	sd	s0,0(sp)
   106d2:	0800                	addi	s0,sp,16
   106d4:	6110                	ld	a2,0(a0)
   106d6:	6514                	ld	a3,8(a0)
   106d8:	872e                	mv	a4,a1
   106da:	8532                	mv	a0,a2
   106dc:	85b6                	mv	a1,a3
   106de:	863a                	mv	a2,a4
   106e0:	60a2                	ld	ra,8(sp)
   106e2:	6402                	ld	s0,0(sp)
   106e4:	0141                	addi	sp,sp,16
   106e6:	00001317          	auipc	t1,0x1
   106ea:	91a30067          	jr	-1766(t1) # 11000 <_ZN42_$LT$str$u20$as$u20$core..fmt..Display$GT$3fmt17h34b49d6546ae1b87E>

00000000000106ee <_ZN44_$LT$$RF$T$u20$as$u20$core..fmt..Display$GT$3fmt17hc095759edf7bce08E>:
   106ee:	1141                	addi	sp,sp,-16
   106f0:	e406                	sd	ra,8(sp)
   106f2:	e022                	sd	s0,0(sp)
   106f4:	0800                	addi	s0,sp,16
   106f6:	6108                	ld	a0,0(a0)
   106f8:	60a2                	ld	ra,8(sp)
   106fa:	6402                	ld	s0,0(sp)
   106fc:	0141                	addi	sp,sp,16
   106fe:	00000317          	auipc	t1,0x0
   10702:	27c30067          	jr	636(t1) # 1097a <_ZN59_$LT$core..fmt..Arguments$u20$as$u20$core..fmt..Display$GT$3fmt17hf2333fcfd2ea2f0dE>

0000000000010706 <_ZN22buddy_system_allocator4Heap4init17h1d037308586cbd83E>:
   10706:	1141                	addi	sp,sp,-16
   10708:	e406                	sd	ra,8(sp)
   1070a:	e022                	sd	s0,0(sp)
   1070c:	0800                	addi	s0,sp,16
   1070e:	962e                	add	a2,a2,a1
   10710:	059d                	addi	a1,a1,7
   10712:	99e1                	andi	a1,a1,-8
   10714:	ff867e93          	andi	t4,a2,-8
   10718:	12beeb63          	bltu	t4,a1,1084e <.LBB5_20>
   1071c:	4701                	li	a4,0
   1071e:	00858613          	addi	a2,a1,8
   10722:	10cee163          	bltu	t4,a2,10824 <.LBB5_18+0xda>

0000000000010726 <.LBB5_15>:
   10726:	00003617          	auipc	a2,0x3
   1072a:	8fa60613          	addi	a2,a2,-1798 # 13020 <.LCPI5_0>
   1072e:	00063803          	ld	a6,0(a2)

0000000000010732 <.LBB5_16>:
   10732:	00003617          	auipc	a2,0x3
   10736:	8f660613          	addi	a2,a2,-1802 # 13028 <.LCPI5_1>
   1073a:	00063f03          	ld	t5,0(a2)

000000000001073e <.LBB5_17>:
   1073e:	00003617          	auipc	a2,0x3
   10742:	8f260613          	addi	a2,a2,-1806 # 13030 <.LCPI5_2>
   10746:	00063883          	ld	a7,0(a2)

000000000001074a <.LBB5_18>:
   1074a:	00003617          	auipc	a2,0x3
   1074e:	8ee60613          	addi	a2,a2,-1810 # 13038 <.LCPI5_3>
   10752:	00063283          	ld	t0,0(a2)
   10756:	03f00313          	li	t1,63
   1075a:	4385                	li	t2,1
   1075c:	4e7d                	li	t3,31
   1075e:	40be8633          	sub	a2,t4,a1
   10762:	ca31                	beqz	a2,107b6 <.LBB5_18+0x6c>
   10764:	00165693          	srli	a3,a2,0x1
   10768:	8e55                	or	a2,a2,a3
   1076a:	00265693          	srli	a3,a2,0x2
   1076e:	8e55                	or	a2,a2,a3
   10770:	00465693          	srli	a3,a2,0x4
   10774:	8e55                	or	a2,a2,a3
   10776:	00865693          	srli	a3,a2,0x8
   1077a:	8e55                	or	a2,a2,a3
   1077c:	01065693          	srli	a3,a2,0x10
   10780:	8e55                	or	a2,a2,a3
   10782:	02065693          	srli	a3,a2,0x20
   10786:	8e55                	or	a2,a2,a3
   10788:	fff64613          	not	a2,a2
   1078c:	00165693          	srli	a3,a2,0x1
   10790:	0106f6b3          	and	a3,a3,a6
   10794:	8e15                	sub	a2,a2,a3
   10796:	01e676b3          	and	a3,a2,t5
   1079a:	8209                	srli	a2,a2,0x2
   1079c:	01e67633          	and	a2,a2,t5
   107a0:	9636                	add	a2,a2,a3
   107a2:	00465693          	srli	a3,a2,0x4
   107a6:	9636                	add	a2,a2,a3
   107a8:	01167633          	and	a2,a2,a7
   107ac:	02560633          	mul	a2,a2,t0
   107b0:	03865693          	srli	a3,a2,0x38
   107b4:	a019                	j	107ba <.LBB5_18+0x70>
   107b6:	04000693          	li	a3,64
   107ba:	40b00633          	neg	a2,a1
   107be:	8e6d                	and	a2,a2,a1
   107c0:	40d306b3          	sub	a3,t1,a3
   107c4:	00d396b3          	sll	a3,t2,a3
   107c8:	00d66363          	bltu	a2,a3,107ce <.LBB5_18+0x84>
   107cc:	8636                	mv	a2,a3
   107ce:	ce05                	beqz	a2,10806 <.LBB5_18+0xbc>
   107d0:	fff60693          	addi	a3,a2,-1
   107d4:	fff64793          	not	a5,a2
   107d8:	8efd                	and	a3,a3,a5
   107da:	0016d793          	srli	a5,a3,0x1
   107de:	0107f7b3          	and	a5,a5,a6
   107e2:	8e9d                	sub	a3,a3,a5
   107e4:	01e6f7b3          	and	a5,a3,t5
   107e8:	8289                	srli	a3,a3,0x2
   107ea:	01e6f6b3          	and	a3,a3,t5
   107ee:	96be                	add	a3,a3,a5
   107f0:	0046d793          	srli	a5,a3,0x4
   107f4:	96be                	add	a3,a3,a5
   107f6:	0116f6b3          	and	a3,a3,a7
   107fa:	025686b3          	mul	a3,a3,t0
   107fe:	92e1                	srli	a3,a3,0x38
   10800:	00de7763          	bgeu	t3,a3,1080e <.LBB5_18+0xc4>
   10804:	a80d                	j	10836 <.LBB5_19>
   10806:	04000693          	li	a3,64
   1080a:	02de6663          	bltu	t3,a3,10836 <.LBB5_19>
   1080e:	068e                	slli	a3,a3,0x3
   10810:	96aa                	add	a3,a3,a0
   10812:	629c                	ld	a5,0(a3)
   10814:	e19c                	sd	a5,0(a1)
   10816:	e28c                	sd	a1,0(a3)
   10818:	95b2                	add	a1,a1,a2
   1081a:	00858693          	addi	a3,a1,8
   1081e:	9732                	add	a4,a4,a2
   10820:	f2deffe3          	bgeu	t4,a3,1075e <.LBB5_18+0x14>
   10824:	11053583          	ld	a1,272(a0)
   10828:	95ba                	add	a1,a1,a4
   1082a:	10b53823          	sd	a1,272(a0)
   1082e:	60a2                	ld	ra,8(sp)
   10830:	6402                	ld	s0,0(sp)
   10832:	0141                	addi	sp,sp,16
   10834:	8082                	ret

0000000000010836 <.LBB5_19>:
   10836:	00002617          	auipc	a2,0x2
   1083a:	c2a60613          	addi	a2,a2,-982 # 12460 <.Lanon.8d83386b1e96181169c0e194ecb55dd8.3>
   1083e:	02000593          	li	a1,32
   10842:	8536                	mv	a0,a3
   10844:	00000097          	auipc	ra,0x0
   10848:	0aa080e7          	jalr	170(ra) # 108ee <_ZN4core9panicking18panic_bounds_check17h44e21cbce79fc049E>
	...

000000000001084e <.LBB5_20>:
   1084e:	00002517          	auipc	a0,0x2
   10852:	b7250513          	addi	a0,a0,-1166 # 123c0 <.Lanon.8d83386b1e96181169c0e194ecb55dd8.0>

0000000000010856 <.LBB5_21>:
   10856:	00002617          	auipc	a2,0x2
   1085a:	bf260613          	addi	a2,a2,-1038 # 12448 <.Lanon.8d83386b1e96181169c0e194ecb55dd8.2>
   1085e:	45f9                	li	a1,30
   10860:	00000097          	auipc	ra,0x0
   10864:	062080e7          	jalr	98(ra) # 108c2 <_ZN4core9panicking5panic17h0ed902ae04a4e669E>
	...

000000000001086a <_ZN78_$LT$buddy_system_allocator..LockedHeap$u20$as$u20$core..ops..deref..Deref$GT$5deref17h009de595a571d1f5E>:
   1086a:	1141                	addi	sp,sp,-16
   1086c:	e406                	sd	ra,8(sp)
   1086e:	e022                	sd	s0,0(sp)
   10870:	0800                	addi	s0,sp,16
   10872:	60a2                	ld	ra,8(sp)
   10874:	6402                	ld	s0,0(sp)
   10876:	0141                	addi	sp,sp,16
   10878:	8082                	ret

000000000001087a <_ZN4core3ops8function6FnOnce9call_once17hf008b2228a16ba81E>:
   1087a:	6108                	ld	a0,0(a0)
   1087c:	a001                	j	1087c <_ZN4core3ops8function6FnOnce9call_once17hf008b2228a16ba81E+0x2>

000000000001087e <_ZN4core3ptr102drop_in_place$LT$$RF$core..iter..adapters..copied..Copied$LT$core..slice..iter..Iter$LT$u8$GT$$GT$$GT$17hf5b0193b586cfa2dE>:
   1087e:	8082                	ret

0000000000010880 <_ZN36_$LT$T$u20$as$u20$core..any..Any$GT$7type_id17h2a8af4a3cc130a60E>:
   10880:	00003517          	auipc	a0,0x3
   10884:	90850513          	addi	a0,a0,-1784 # 13188 <.LCPI97_0>
   10888:	6108                	ld	a0,0(a0)
   1088a:	8082                	ret

000000000001088c <_ZN4core5panic10panic_info9PanicInfo7message17h8ed89e8633dfae6dE>:
   1088c:	6908                	ld	a0,16(a0)
   1088e:	8082                	ret

0000000000010890 <_ZN4core5panic10panic_info9PanicInfo8location17he3738231e6489566E>:
   10890:	6d08                	ld	a0,24(a0)
   10892:	8082                	ret

0000000000010894 <_ZN4core9panicking9panic_fmt17hdc67c01c17e684eeE>:
   10894:	7179                	addi	sp,sp,-48
   10896:	f406                	sd	ra,40(sp)

0000000000010898 <.LBB169_1>:
   10898:	00002617          	auipc	a2,0x2
   1089c:	be060613          	addi	a2,a2,-1056 # 12478 <.Lanon.4ef9c7dd071e32d62657b33c898a56d6.2>
   108a0:	e032                	sd	a2,0(sp)

00000000000108a2 <.LBB169_2>:
   108a2:	00002617          	auipc	a2,0x2
   108a6:	c2e60613          	addi	a2,a2,-978 # 124d0 <.Lanon.4ef9c7dd071e32d62657b33c898a56d6.231>
   108aa:	e432                	sd	a2,8(sp)
   108ac:	e82a                	sd	a0,16(sp)
   108ae:	ec2e                	sd	a1,24(sp)
   108b0:	4505                	li	a0,1
   108b2:	02a10023          	sb	a0,32(sp)
   108b6:	850a                	mv	a0,sp
   108b8:	00000097          	auipc	ra,0x0
   108bc:	cb4080e7          	jalr	-844(ra) # 1056c <rust_begin_unwind>
	...

00000000000108c2 <_ZN4core9panicking5panic17h0ed902ae04a4e669E>:
   108c2:	715d                	addi	sp,sp,-80
   108c4:	e486                	sd	ra,72(sp)
   108c6:	fc2a                	sd	a0,56(sp)
   108c8:	e0ae                	sd	a1,64(sp)
   108ca:	1828                	addi	a0,sp,56
   108cc:	ec2a                	sd	a0,24(sp)
   108ce:	4505                	li	a0,1
   108d0:	f02a                	sd	a0,32(sp)
   108d2:	e402                	sd	zero,8(sp)

00000000000108d4 <.LBB171_1>:
   108d4:	00002517          	auipc	a0,0x2
   108d8:	ba450513          	addi	a0,a0,-1116 # 12478 <.Lanon.4ef9c7dd071e32d62657b33c898a56d6.2>
   108dc:	f42a                	sd	a0,40(sp)
   108de:	f802                	sd	zero,48(sp)
   108e0:	0028                	addi	a0,sp,8
   108e2:	85b2                	mv	a1,a2
   108e4:	00000097          	auipc	ra,0x0
   108e8:	fb0080e7          	jalr	-80(ra) # 10894 <_ZN4core9panicking9panic_fmt17hdc67c01c17e684eeE>
	...

00000000000108ee <_ZN4core9panicking18panic_bounds_check17h44e21cbce79fc049E>:
   108ee:	7159                	addi	sp,sp,-112
   108f0:	f486                	sd	ra,104(sp)
   108f2:	e42a                	sd	a0,8(sp)
   108f4:	e82e                	sd	a1,16(sp)
   108f6:	0808                	addi	a0,sp,16
   108f8:	e4aa                	sd	a0,72(sp)

00000000000108fa <.LBB175_1>:
   108fa:	00001517          	auipc	a0,0x1
   108fe:	a5450513          	addi	a0,a0,-1452 # 1134e <_ZN4core3fmt3num3imp52_$LT$impl$u20$core..fmt..Display$u20$for$u20$u64$GT$3fmt17h637b75f18de6578eE>
   10902:	e8aa                	sd	a0,80(sp)
   10904:	002c                	addi	a1,sp,8
   10906:	ecae                	sd	a1,88(sp)
   10908:	f0aa                	sd	a0,96(sp)

000000000001090a <.LBB175_2>:
   1090a:	00002517          	auipc	a0,0x2
   1090e:	ba650513          	addi	a0,a0,-1114 # 124b0 <.Lanon.4ef9c7dd071e32d62657b33c898a56d6.206>
   10912:	f42a                	sd	a0,40(sp)
   10914:	4509                	li	a0,2
   10916:	f82a                	sd	a0,48(sp)
   10918:	ec02                	sd	zero,24(sp)
   1091a:	00ac                	addi	a1,sp,72
   1091c:	fc2e                	sd	a1,56(sp)
   1091e:	e0aa                	sd	a0,64(sp)
   10920:	0828                	addi	a0,sp,24
   10922:	85b2                	mv	a1,a2
   10924:	00000097          	auipc	ra,0x0
   10928:	f70080e7          	jalr	-144(ra) # 10894 <_ZN4core9panicking9panic_fmt17hdc67c01c17e684eeE>
	...

000000000001092e <_ZN4core6result13unwrap_failed17h1ccf3470a5333e86E>:
   1092e:	7119                	addi	sp,sp,-128
   10930:	fc86                	sd	ra,120(sp)
   10932:	e42a                	sd	a0,8(sp)
   10934:	e82e                	sd	a1,16(sp)
   10936:	ec32                	sd	a2,24(sp)
   10938:	f036                	sd	a3,32(sp)
   1093a:	0028                	addi	a0,sp,8
   1093c:	ecaa                	sd	a0,88(sp)

000000000001093e <.LBB182_1>:
   1093e:	00001517          	auipc	a0,0x1
   10942:	a3850513          	addi	a0,a0,-1480 # 11376 <_ZN44_$LT$$RF$T$u20$as$u20$core..fmt..Display$GT$3fmt17hb77f5e04fcec7777E>
   10946:	f0aa                	sd	a0,96(sp)
   10948:	0828                	addi	a0,sp,24
   1094a:	f4aa                	sd	a0,104(sp)

000000000001094c <.LBB182_2>:
   1094c:	00001517          	auipc	a0,0x1
   10950:	a2250513          	addi	a0,a0,-1502 # 1136e <_ZN42_$LT$$RF$T$u20$as$u20$core..fmt..Debug$GT$3fmt17h7acd352b070da26eE>
   10954:	f8aa                	sd	a0,112(sp)

0000000000010956 <.LBB182_3>:
   10956:	00002517          	auipc	a0,0x2
   1095a:	ba250513          	addi	a0,a0,-1118 # 124f8 <.Lanon.4ef9c7dd071e32d62657b33c898a56d6.250>
   1095e:	fc2a                	sd	a0,56(sp)
   10960:	4509                	li	a0,2
   10962:	e0aa                	sd	a0,64(sp)
   10964:	f402                	sd	zero,40(sp)
   10966:	08ac                	addi	a1,sp,88
   10968:	e4ae                	sd	a1,72(sp)
   1096a:	e8aa                	sd	a0,80(sp)
   1096c:	1028                	addi	a0,sp,40
   1096e:	85ba                	mv	a1,a4
   10970:	00000097          	auipc	ra,0x0
   10974:	f24080e7          	jalr	-220(ra) # 10894 <_ZN4core9panicking9panic_fmt17hdc67c01c17e684eeE>
	...

000000000001097a <_ZN59_$LT$core..fmt..Arguments$u20$as$u20$core..fmt..Display$GT$3fmt17hf2333fcfd2ea2f0dE>:
   1097a:	7139                	addi	sp,sp,-64
   1097c:	fc06                	sd	ra,56(sp)
   1097e:	7510                	ld	a2,40(a0)
   10980:	7118                	ld	a4,32(a0)
   10982:	6d1c                	ld	a5,24(a0)
   10984:	f832                	sd	a2,48(sp)
   10986:	6194                	ld	a3,0(a1)
   10988:	f43a                	sd	a4,40(sp)
   1098a:	f03e                	sd	a5,32(sp)
   1098c:	6910                	ld	a2,16(a0)
   1098e:	6518                	ld	a4,8(a0)
   10990:	6108                	ld	a0,0(a0)
   10992:	658c                	ld	a1,8(a1)
   10994:	ec32                	sd	a2,24(sp)
   10996:	e83a                	sd	a4,16(sp)
   10998:	e42a                	sd	a0,8(sp)
   1099a:	0030                	addi	a2,sp,8
   1099c:	8536                	mv	a0,a3
   1099e:	00000097          	auipc	ra,0x0
   109a2:	00e080e7          	jalr	14(ra) # 109ac <_ZN4core3fmt5write17h9d53518631e11158E>
   109a6:	70e2                	ld	ra,56(sp)
   109a8:	6121                	addi	sp,sp,64
   109aa:	8082                	ret

00000000000109ac <_ZN4core3fmt5write17h9d53518631e11158E>:
   109ac:	7119                	addi	sp,sp,-128
   109ae:	fc86                	sd	ra,120(sp)
   109b0:	f8a2                	sd	s0,112(sp)
   109b2:	f4a6                	sd	s1,104(sp)
   109b4:	f0ca                	sd	s2,96(sp)
   109b6:	ecce                	sd	s3,88(sp)
   109b8:	e8d2                	sd	s4,80(sp)
   109ba:	e4d6                	sd	s5,72(sp)
   109bc:	e0da                	sd	s6,64(sp)
   109be:	89b2                	mv	s3,a2
   109c0:	4605                	li	a2,1
   109c2:	1616                	slli	a2,a2,0x25
   109c4:	f832                	sd	a2,48(sp)
   109c6:	460d                	li	a2,3
   109c8:	02c10c23          	sb	a2,56(sp)
   109cc:	0009b603          	ld	a2,0(s3)
   109d0:	e802                	sd	zero,16(sp)
   109d2:	f002                	sd	zero,32(sp)
   109d4:	e02a                	sd	a0,0(sp)
   109d6:	e42e                	sd	a1,8(sp)
   109d8:	c669                	beqz	a2,10aa2 <.LBB219_31+0x9e>
   109da:	0089b503          	ld	a0,8(s3)
   109de:	10050e63          	beqz	a0,10afa <.LBB219_31+0xf6>
   109e2:	0109b583          	ld	a1,16(s3)
   109e6:	fff50693          	addi	a3,a0,-1
   109ea:	068e                	slli	a3,a3,0x3
   109ec:	828d                	srli	a3,a3,0x3
   109ee:	00168913          	addi	s2,a3,1
   109f2:	00858493          	addi	s1,a1,8
   109f6:	03800593          	li	a1,56
   109fa:	02b50a33          	mul	s4,a0,a1
   109fe:	01860413          	addi	s0,a2,24
   10a02:	4a85                	li	s5,1

0000000000010a04 <.LBB219_31>:
   10a04:	00000b17          	auipc	s6,0x0
   10a08:	e76b0b13          	addi	s6,s6,-394 # 1087a <_ZN4core3ops8function6FnOnce9call_once17hf008b2228a16ba81E>
   10a0c:	6090                	ld	a2,0(s1)
   10a0e:	ca09                	beqz	a2,10a20 <.LBB219_31+0x1c>
   10a10:	66a2                	ld	a3,8(sp)
   10a12:	6502                	ld	a0,0(sp)
   10a14:	ff84b583          	ld	a1,-8(s1)
   10a18:	6e94                	ld	a3,24(a3)
   10a1a:	9682                	jalr	a3
   10a1c:	10051163          	bnez	a0,10b1e <.LBB219_31+0x11a>
   10a20:	4448                	lw	a0,12(s0)
   10a22:	da2a                	sw	a0,52(sp)
   10a24:	01040503          	lb	a0,16(s0)
   10a28:	02a10c23          	sb	a0,56(sp)
   10a2c:	440c                	lw	a1,8(s0)
   10a2e:	0209b503          	ld	a0,32(s3)
   10a32:	d82e                	sw	a1,48(sp)
   10a34:	ff843683          	ld	a3,-8(s0)
   10a38:	600c                	ld	a1,0(s0)
   10a3a:	ce89                	beqz	a3,10a54 <.LBB219_31+0x50>
   10a3c:	4601                	li	a2,0
   10a3e:	01569c63          	bne	a3,s5,10a56 <.LBB219_31+0x52>
   10a42:	0592                	slli	a1,a1,0x4
   10a44:	95aa                	add	a1,a1,a0
   10a46:	6590                	ld	a2,8(a1)
   10a48:	01660463          	beq	a2,s6,10a50 <.LBB219_31+0x4c>
   10a4c:	4601                	li	a2,0
   10a4e:	a021                	j	10a56 <.LBB219_31+0x52>
   10a50:	618c                	ld	a1,0(a1)
   10a52:	618c                	ld	a1,0(a1)
   10a54:	4605                	li	a2,1
   10a56:	e832                	sd	a2,16(sp)
   10a58:	ec2e                	sd	a1,24(sp)
   10a5a:	fe843683          	ld	a3,-24(s0)
   10a5e:	ff043583          	ld	a1,-16(s0)
   10a62:	ce89                	beqz	a3,10a7c <.LBB219_31+0x78>
   10a64:	4601                	li	a2,0
   10a66:	01569c63          	bne	a3,s5,10a7e <.LBB219_31+0x7a>
   10a6a:	0592                	slli	a1,a1,0x4
   10a6c:	95aa                	add	a1,a1,a0
   10a6e:	6590                	ld	a2,8(a1)
   10a70:	01660463          	beq	a2,s6,10a78 <.LBB219_31+0x74>
   10a74:	4601                	li	a2,0
   10a76:	a021                	j	10a7e <.LBB219_31+0x7a>
   10a78:	618c                	ld	a1,0(a1)
   10a7a:	618c                	ld	a1,0(a1)
   10a7c:	4605                	li	a2,1
   10a7e:	f032                	sd	a2,32(sp)
   10a80:	f42e                	sd	a1,40(sp)
   10a82:	6c0c                	ld	a1,24(s0)
   10a84:	0592                	slli	a1,a1,0x4
   10a86:	952e                	add	a0,a0,a1
   10a88:	6510                	ld	a2,8(a0)
   10a8a:	6108                	ld	a0,0(a0)
   10a8c:	858a                	mv	a1,sp
   10a8e:	9602                	jalr	a2
   10a90:	e559                	bnez	a0,10b1e <.LBB219_31+0x11a>
   10a92:	04c1                	addi	s1,s1,16
   10a94:	fc8a0a13          	addi	s4,s4,-56
   10a98:	03840413          	addi	s0,s0,56
   10a9c:	f60a18e3          	bnez	s4,10a0c <.LBB219_31+0x8>
   10aa0:	a881                	j	10af0 <.LBB219_31+0xec>
   10aa2:	0289b503          	ld	a0,40(s3)
   10aa6:	c931                	beqz	a0,10afa <.LBB219_31+0xf6>
   10aa8:	0209b583          	ld	a1,32(s3)
   10aac:	0109b603          	ld	a2,16(s3)
   10ab0:	fff50693          	addi	a3,a0,-1
   10ab4:	0692                	slli	a3,a3,0x4
   10ab6:	8291                	srli	a3,a3,0x4
   10ab8:	00168913          	addi	s2,a3,1
   10abc:	00860413          	addi	s0,a2,8
   10ac0:	00451a13          	slli	s4,a0,0x4
   10ac4:	00858493          	addi	s1,a1,8
   10ac8:	6010                	ld	a2,0(s0)
   10aca:	ca01                	beqz	a2,10ada <.LBB219_31+0xd6>
   10acc:	66a2                	ld	a3,8(sp)
   10ace:	6502                	ld	a0,0(sp)
   10ad0:	ff843583          	ld	a1,-8(s0)
   10ad4:	6e94                	ld	a3,24(a3)
   10ad6:	9682                	jalr	a3
   10ad8:	e139                	bnez	a0,10b1e <.LBB219_31+0x11a>
   10ada:	6090                	ld	a2,0(s1)
   10adc:	ff84b503          	ld	a0,-8(s1)
   10ae0:	858a                	mv	a1,sp
   10ae2:	9602                	jalr	a2
   10ae4:	ed0d                	bnez	a0,10b1e <.LBB219_31+0x11a>
   10ae6:	0441                	addi	s0,s0,16
   10ae8:	1a41                	addi	s4,s4,-16
   10aea:	04c1                	addi	s1,s1,16
   10aec:	fc0a1ee3          	bnez	s4,10ac8 <.LBB219_31+0xc4>
   10af0:	0189b503          	ld	a0,24(s3)
   10af4:	00a96863          	bltu	s2,a0,10b04 <.LBB219_31+0x100>
   10af8:	a02d                	j	10b22 <.LBB219_31+0x11e>
   10afa:	4901                	li	s2,0
   10afc:	0189b503          	ld	a0,24(s3)
   10b00:	02a97163          	bgeu	s2,a0,10b22 <.LBB219_31+0x11e>
   10b04:	0109b503          	ld	a0,16(s3)
   10b08:	00491593          	slli	a1,s2,0x4
   10b0c:	00b50633          	add	a2,a0,a1
   10b10:	66a2                	ld	a3,8(sp)
   10b12:	6502                	ld	a0,0(sp)
   10b14:	620c                	ld	a1,0(a2)
   10b16:	6610                	ld	a2,8(a2)
   10b18:	6e94                	ld	a3,24(a3)
   10b1a:	9682                	jalr	a3
   10b1c:	c119                	beqz	a0,10b22 <.LBB219_31+0x11e>
   10b1e:	4505                	li	a0,1
   10b20:	a011                	j	10b24 <.LBB219_31+0x120>
   10b22:	4501                	li	a0,0
   10b24:	70e6                	ld	ra,120(sp)
   10b26:	7446                	ld	s0,112(sp)
   10b28:	74a6                	ld	s1,104(sp)
   10b2a:	7906                	ld	s2,96(sp)
   10b2c:	69e6                	ld	s3,88(sp)
   10b2e:	6a46                	ld	s4,80(sp)
   10b30:	6aa6                	ld	s5,72(sp)
   10b32:	6b06                	ld	s6,64(sp)
   10b34:	6109                	addi	sp,sp,128
   10b36:	8082                	ret

0000000000010b38 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E>:
   10b38:	7159                	addi	sp,sp,-112
   10b3a:	f486                	sd	ra,104(sp)
   10b3c:	f0a2                	sd	s0,96(sp)
   10b3e:	eca6                	sd	s1,88(sp)
   10b40:	e8ca                	sd	s2,80(sp)
   10b42:	e4ce                	sd	s3,72(sp)
   10b44:	e0d2                	sd	s4,64(sp)
   10b46:	fc56                	sd	s5,56(sp)
   10b48:	f85a                	sd	s6,48(sp)
   10b4a:	f45e                	sd	s7,40(sp)
   10b4c:	f062                	sd	s8,32(sp)
   10b4e:	ec66                	sd	s9,24(sp)
   10b50:	e86a                	sd	s10,16(sp)
   10b52:	e46e                	sd	s11,8(sp)
   10b54:	89be                	mv	s3,a5
   10b56:	893a                	mv	s2,a4
   10b58:	8b36                	mv	s6,a3
   10b5a:	8a32                	mv	s4,a2
   10b5c:	8c2a                	mv	s8,a0
   10b5e:	c1b9                	beqz	a1,10ba4 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x6c>
   10b60:	030c6403          	lwu	s0,48(s8)
   10b64:	00147513          	andi	a0,s0,1
   10b68:	00110ab7          	lui	s5,0x110
   10b6c:	c119                	beqz	a0,10b72 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x3a>
   10b6e:	02b00a93          	li	s5,43
   10b72:	01350cb3          	add	s9,a0,s3
   10b76:	00447513          	andi	a0,s0,4
   10b7a:	cd15                	beqz	a0,10bb6 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x7e>
   10b7c:	02000513          	li	a0,32
   10b80:	04ab7063          	bgeu	s6,a0,10bc0 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x88>
   10b84:	4501                	li	a0,0
   10b86:	040b0363          	beqz	s6,10bcc <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x94>
   10b8a:	85da                	mv	a1,s6
   10b8c:	8652                	mv	a2,s4
   10b8e:	00060683          	lb	a3,0(a2)
   10b92:	0605                	addi	a2,a2,1
   10b94:	fc06a693          	slti	a3,a3,-64
   10b98:	0016c693          	xori	a3,a3,1
   10b9c:	15fd                	addi	a1,a1,-1
   10b9e:	9536                	add	a0,a0,a3
   10ba0:	f5fd                	bnez	a1,10b8e <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x56>
   10ba2:	a02d                	j	10bcc <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x94>
   10ba4:	030c2403          	lw	s0,48(s8)
   10ba8:	00198c93          	addi	s9,s3,1
   10bac:	02d00a93          	li	s5,45
   10bb0:	00447513          	andi	a0,s0,4
   10bb4:	f561                	bnez	a0,10b7c <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x44>
   10bb6:	4a01                	li	s4,0
   10bb8:	010c3503          	ld	a0,16(s8)
   10bbc:	ed01                	bnez	a0,10bd4 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x9c>
   10bbe:	a099                	j	10c04 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xcc>
   10bc0:	8552                	mv	a0,s4
   10bc2:	85da                	mv	a1,s6
   10bc4:	00000097          	auipc	ra,0x0
   10bc8:	44c080e7          	jalr	1100(ra) # 11010 <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE>
   10bcc:	9caa                	add	s9,s9,a0
   10bce:	010c3503          	ld	a0,16(s8)
   10bd2:	c90d                	beqz	a0,10c04 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xcc>
   10bd4:	018c3d03          	ld	s10,24(s8)
   10bd8:	03acf663          	bgeu	s9,s10,10c04 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xcc>
   10bdc:	00847513          	andi	a0,s0,8
   10be0:	e541                	bnez	a0,10c68 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x130>
   10be2:	038c4583          	lbu	a1,56(s8)
   10be6:	460d                	li	a2,3
   10be8:	4505                	li	a0,1
   10bea:	00c58363          	beq	a1,a2,10bf0 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xb8>
   10bee:	852e                	mv	a0,a1
   10bf0:	00357593          	andi	a1,a0,3
   10bf4:	419d0533          	sub	a0,s10,s9
   10bf8:	c1e1                	beqz	a1,10cb8 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x180>
   10bfa:	4605                	li	a2,1
   10bfc:	0cc59163          	bne	a1,a2,10cbe <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x186>
   10c00:	4d01                	li	s10,0
   10c02:	a0d9                	j	10cc8 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x190>
   10c04:	000c3403          	ld	s0,0(s8)
   10c08:	008c3483          	ld	s1,8(s8)
   10c0c:	8522                	mv	a0,s0
   10c0e:	85a6                	mv	a1,s1
   10c10:	8656                	mv	a2,s5
   10c12:	86d2                	mv	a3,s4
   10c14:	875a                	mv	a4,s6
   10c16:	00000097          	auipc	ra,0x0
   10c1a:	140080e7          	jalr	320(ra) # 10d56 <_ZN4core3fmt9Formatter12pad_integral12write_prefix17hd3daa56e213ffe72E>
   10c1e:	4b85                	li	s7,1
   10c20:	c10d                	beqz	a0,10c42 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x10a>
   10c22:	855e                	mv	a0,s7
   10c24:	70a6                	ld	ra,104(sp)
   10c26:	7406                	ld	s0,96(sp)
   10c28:	64e6                	ld	s1,88(sp)
   10c2a:	6946                	ld	s2,80(sp)
   10c2c:	69a6                	ld	s3,72(sp)
   10c2e:	6a06                	ld	s4,64(sp)
   10c30:	7ae2                	ld	s5,56(sp)
   10c32:	7b42                	ld	s6,48(sp)
   10c34:	7ba2                	ld	s7,40(sp)
   10c36:	7c02                	ld	s8,32(sp)
   10c38:	6ce2                	ld	s9,24(sp)
   10c3a:	6d42                	ld	s10,16(sp)
   10c3c:	6da2                	ld	s11,8(sp)
   10c3e:	6165                	addi	sp,sp,112
   10c40:	8082                	ret
   10c42:	6c9c                	ld	a5,24(s1)
   10c44:	8522                	mv	a0,s0
   10c46:	85ca                	mv	a1,s2
   10c48:	864e                	mv	a2,s3
   10c4a:	70a6                	ld	ra,104(sp)
   10c4c:	7406                	ld	s0,96(sp)
   10c4e:	64e6                	ld	s1,88(sp)
   10c50:	6946                	ld	s2,80(sp)
   10c52:	69a6                	ld	s3,72(sp)
   10c54:	6a06                	ld	s4,64(sp)
   10c56:	7ae2                	ld	s5,56(sp)
   10c58:	7b42                	ld	s6,48(sp)
   10c5a:	7ba2                	ld	s7,40(sp)
   10c5c:	7c02                	ld	s8,32(sp)
   10c5e:	6ce2                	ld	s9,24(sp)
   10c60:	6d42                	ld	s10,16(sp)
   10c62:	6da2                	ld	s11,8(sp)
   10c64:	6165                	addi	sp,sp,112
   10c66:	8782                	jr	a5
   10c68:	034c2403          	lw	s0,52(s8)
   10c6c:	03000513          	li	a0,48
   10c70:	038c4583          	lbu	a1,56(s8)
   10c74:	e02e                	sd	a1,0(sp)
   10c76:	000c3d83          	ld	s11,0(s8)
   10c7a:	008c3483          	ld	s1,8(s8)
   10c7e:	02ac2a23          	sw	a0,52(s8)
   10c82:	4b85                	li	s7,1
   10c84:	037c0c23          	sb	s7,56(s8)
   10c88:	856e                	mv	a0,s11
   10c8a:	85a6                	mv	a1,s1
   10c8c:	8656                	mv	a2,s5
   10c8e:	86d2                	mv	a3,s4
   10c90:	875a                	mv	a4,s6
   10c92:	00000097          	auipc	ra,0x0
   10c96:	0c4080e7          	jalr	196(ra) # 10d56 <_ZN4core3fmt9Formatter12pad_integral12write_prefix17hd3daa56e213ffe72E>
   10c9a:	f541                	bnez	a0,10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>
   10c9c:	8a22                	mv	s4,s0
   10c9e:	419d0533          	sub	a0,s10,s9
   10ca2:	00150413          	addi	s0,a0,1
   10ca6:	147d                	addi	s0,s0,-1
   10ca8:	c449                	beqz	s0,10d32 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x1fa>
   10caa:	7090                	ld	a2,32(s1)
   10cac:	03000593          	li	a1,48
   10cb0:	856e                	mv	a0,s11
   10cb2:	9602                	jalr	a2
   10cb4:	d96d                	beqz	a0,10ca6 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x16e>
   10cb6:	b7b5                	j	10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>
   10cb8:	8d2a                	mv	s10,a0
   10cba:	852e                	mv	a0,a1
   10cbc:	a031                	j	10cc8 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x190>
   10cbe:	00150593          	addi	a1,a0,1
   10cc2:	8105                	srli	a0,a0,0x1
   10cc4:	0015dd13          	srli	s10,a1,0x1
   10cc8:	000c3c83          	ld	s9,0(s8)
   10ccc:	008c3d83          	ld	s11,8(s8)
   10cd0:	034c2403          	lw	s0,52(s8)
   10cd4:	00150493          	addi	s1,a0,1
   10cd8:	14fd                	addi	s1,s1,-1
   10cda:	c889                	beqz	s1,10cec <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x1b4>
   10cdc:	020db603          	ld	a2,32(s11)
   10ce0:	8566                	mv	a0,s9
   10ce2:	85a2                	mv	a1,s0
   10ce4:	9602                	jalr	a2
   10ce6:	d96d                	beqz	a0,10cd8 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x1a0>
   10ce8:	4b85                	li	s7,1
   10cea:	bf25                	j	10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>
   10cec:	00110537          	lui	a0,0x110
   10cf0:	4b85                	li	s7,1
   10cf2:	f2a408e3          	beq	s0,a0,10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>
   10cf6:	8566                	mv	a0,s9
   10cf8:	85ee                	mv	a1,s11
   10cfa:	8656                	mv	a2,s5
   10cfc:	86d2                	mv	a3,s4
   10cfe:	875a                	mv	a4,s6
   10d00:	00000097          	auipc	ra,0x0
   10d04:	056080e7          	jalr	86(ra) # 10d56 <_ZN4core3fmt9Formatter12pad_integral12write_prefix17hd3daa56e213ffe72E>
   10d08:	fd09                	bnez	a0,10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>
   10d0a:	018db683          	ld	a3,24(s11)
   10d0e:	8566                	mv	a0,s9
   10d10:	85ca                	mv	a1,s2
   10d12:	864e                	mv	a2,s3
   10d14:	9682                	jalr	a3
   10d16:	f511                	bnez	a0,10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>
   10d18:	4481                	li	s1,0
   10d1a:	029d0a63          	beq	s10,s1,10d4e <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x216>
   10d1e:	020db603          	ld	a2,32(s11)
   10d22:	0485                	addi	s1,s1,1
   10d24:	8566                	mv	a0,s9
   10d26:	85a2                	mv	a1,s0
   10d28:	9602                	jalr	a2
   10d2a:	d965                	beqz	a0,10d1a <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x1e2>
   10d2c:	fff48513          	addi	a0,s1,-1
   10d30:	a005                	j	10d50 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0x218>
   10d32:	6c94                	ld	a3,24(s1)
   10d34:	856e                	mv	a0,s11
   10d36:	85ca                	mv	a1,s2
   10d38:	864e                	mv	a2,s3
   10d3a:	9682                	jalr	a3
   10d3c:	ee0513e3          	bnez	a0,10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>
   10d40:	4b81                	li	s7,0
   10d42:	034c2a23          	sw	s4,52(s8)
   10d46:	6502                	ld	a0,0(sp)
   10d48:	02ac0c23          	sb	a0,56(s8)
   10d4c:	bdd9                	j	10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>
   10d4e:	856a                	mv	a0,s10
   10d50:	01a53bb3          	sltu	s7,a0,s10
   10d54:	b5f9                	j	10c22 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E+0xea>

0000000000010d56 <_ZN4core3fmt9Formatter12pad_integral12write_prefix17hd3daa56e213ffe72E>:
   10d56:	7179                	addi	sp,sp,-48
   10d58:	f406                	sd	ra,40(sp)
   10d5a:	f022                	sd	s0,32(sp)
   10d5c:	ec26                	sd	s1,24(sp)
   10d5e:	e84a                	sd	s2,16(sp)
   10d60:	e44e                	sd	s3,8(sp)
   10d62:	0006079b          	sext.w	a5,a2
   10d66:	00110837          	lui	a6,0x110
   10d6a:	893a                	mv	s2,a4
   10d6c:	84b6                	mv	s1,a3
   10d6e:	842e                	mv	s0,a1
   10d70:	89aa                	mv	s3,a0
   10d72:	01078963          	beq	a5,a6,10d84 <_ZN4core3fmt9Formatter12pad_integral12write_prefix17hd3daa56e213ffe72E+0x2e>
   10d76:	7014                	ld	a3,32(s0)
   10d78:	854e                	mv	a0,s3
   10d7a:	85b2                	mv	a1,a2
   10d7c:	9682                	jalr	a3
   10d7e:	85aa                	mv	a1,a0
   10d80:	4505                	li	a0,1
   10d82:	ed91                	bnez	a1,10d9e <_ZN4core3fmt9Formatter12pad_integral12write_prefix17hd3daa56e213ffe72E+0x48>
   10d84:	cc81                	beqz	s1,10d9c <_ZN4core3fmt9Formatter12pad_integral12write_prefix17hd3daa56e213ffe72E+0x46>
   10d86:	6c1c                	ld	a5,24(s0)
   10d88:	854e                	mv	a0,s3
   10d8a:	85a6                	mv	a1,s1
   10d8c:	864a                	mv	a2,s2
   10d8e:	70a2                	ld	ra,40(sp)
   10d90:	7402                	ld	s0,32(sp)
   10d92:	64e2                	ld	s1,24(sp)
   10d94:	6942                	ld	s2,16(sp)
   10d96:	69a2                	ld	s3,8(sp)
   10d98:	6145                	addi	sp,sp,48
   10d9a:	8782                	jr	a5
   10d9c:	4501                	li	a0,0
   10d9e:	70a2                	ld	ra,40(sp)
   10da0:	7402                	ld	s0,32(sp)
   10da2:	64e2                	ld	s1,24(sp)
   10da4:	6942                	ld	s2,16(sp)
   10da6:	69a2                	ld	s3,8(sp)
   10da8:	6145                	addi	sp,sp,48
   10daa:	8082                	ret

0000000000010dac <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E>:
   10dac:	715d                	addi	sp,sp,-80
   10dae:	e486                	sd	ra,72(sp)
   10db0:	e0a2                	sd	s0,64(sp)
   10db2:	fc26                	sd	s1,56(sp)
   10db4:	f84a                	sd	s2,48(sp)
   10db6:	f44e                	sd	s3,40(sp)
   10db8:	f052                	sd	s4,32(sp)
   10dba:	ec56                	sd	s5,24(sp)
   10dbc:	e85a                	sd	s6,16(sp)
   10dbe:	e45e                	sd	s7,8(sp)
   10dc0:	8a2a                	mv	s4,a0
   10dc2:	01053283          	ld	t0,16(a0) # 110010 <_ZN8user_lib4HEAP17he3af741f2715d7f2E+0xf8cb8>
   10dc6:	7108                	ld	a0,32(a0)
   10dc8:	fff28693          	addi	a3,t0,-1
   10dcc:	00d036b3          	snez	a3,a3
   10dd0:	fff50713          	addi	a4,a0,-1
   10dd4:	00e03733          	snez	a4,a4
   10dd8:	8ef9                	and	a3,a3,a4
   10dda:	89b2                	mv	s3,a2
   10ddc:	892e                	mv	s2,a1
   10dde:	16069d63          	bnez	a3,10f58 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x1ac>
   10de2:	4585                	li	a1,1
   10de4:	10b51863          	bne	a0,a1,10ef4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x148>
   10de8:	028a3503          	ld	a0,40(s4)
   10dec:	4581                	li	a1,0
   10dee:	013906b3          	add	a3,s2,s3
   10df2:	00150713          	addi	a4,a0,1
   10df6:	00110337          	lui	t1,0x110
   10dfa:	0df00893          	li	a7,223
   10dfe:	0f000813          	li	a6,240
   10e02:	864a                	mv	a2,s2
   10e04:	a801                	j	10e14 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x68>
   10e06:	00160513          	addi	a0,a2,1
   10e0a:	8d91                	sub	a1,a1,a2
   10e0c:	95aa                	add	a1,a1,a0
   10e0e:	862a                	mv	a2,a0
   10e10:	0e640263          	beq	s0,t1,10ef4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x148>
   10e14:	177d                	addi	a4,a4,-1
   10e16:	c725                	beqz	a4,10e7e <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0xd2>
   10e18:	0cd60e63          	beq	a2,a3,10ef4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x148>
   10e1c:	00060503          	lb	a0,0(a2)
   10e20:	0ff57413          	andi	s0,a0,255
   10e24:	fe0551e3          	bgez	a0,10e06 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x5a>
   10e28:	00164503          	lbu	a0,1(a2)
   10e2c:	01f47793          	andi	a5,s0,31
   10e30:	03f57493          	andi	s1,a0,63
   10e34:	0288f963          	bgeu	a7,s0,10e66 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0xba>
   10e38:	00264503          	lbu	a0,2(a2)
   10e3c:	049a                	slli	s1,s1,0x6
   10e3e:	03f57513          	andi	a0,a0,63
   10e42:	8cc9                	or	s1,s1,a0
   10e44:	03046763          	bltu	s0,a6,10e72 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0xc6>
   10e48:	00364503          	lbu	a0,3(a2)
   10e4c:	17f6                	slli	a5,a5,0x3d
   10e4e:	93ad                	srli	a5,a5,0x2b
   10e50:	049a                	slli	s1,s1,0x6
   10e52:	03f57513          	andi	a0,a0,63
   10e56:	8d45                	or	a0,a0,s1
   10e58:	00f56433          	or	s0,a0,a5
   10e5c:	08640c63          	beq	s0,t1,10ef4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x148>
   10e60:	00460513          	addi	a0,a2,4
   10e64:	b75d                	j	10e0a <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x5e>
   10e66:	00260513          	addi	a0,a2,2
   10e6a:	079a                	slli	a5,a5,0x6
   10e6c:	0097e433          	or	s0,a5,s1
   10e70:	bf69                	j	10e0a <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x5e>
   10e72:	00360513          	addi	a0,a2,3
   10e76:	07b2                	slli	a5,a5,0xc
   10e78:	00f4e433          	or	s0,s1,a5
   10e7c:	b779                	j	10e0a <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x5e>
   10e7e:	06d60b63          	beq	a2,a3,10ef4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x148>
   10e82:	00060503          	lb	a0,0(a2)
   10e86:	04055363          	bgez	a0,10ecc <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x120>
   10e8a:	0ff57513          	andi	a0,a0,255
   10e8e:	0e000693          	li	a3,224
   10e92:	02d56d63          	bltu	a0,a3,10ecc <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x120>
   10e96:	0f000693          	li	a3,240
   10e9a:	02d56963          	bltu	a0,a3,10ecc <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x120>
   10e9e:	00164683          	lbu	a3,1(a2)
   10ea2:	00264703          	lbu	a4,2(a2)
   10ea6:	03f6f693          	andi	a3,a3,63
   10eaa:	03f77713          	andi	a4,a4,63
   10eae:	00364603          	lbu	a2,3(a2)
   10eb2:	1576                	slli	a0,a0,0x3d
   10eb4:	912d                	srli	a0,a0,0x2b
   10eb6:	06b2                	slli	a3,a3,0xc
   10eb8:	071a                	slli	a4,a4,0x6
   10eba:	8ed9                	or	a3,a3,a4
   10ebc:	03f67613          	andi	a2,a2,63
   10ec0:	8e55                	or	a2,a2,a3
   10ec2:	8d51                	or	a0,a0,a2
   10ec4:	00110637          	lui	a2,0x110
   10ec8:	02c50663          	beq	a0,a2,10ef4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x148>
   10ecc:	c185                	beqz	a1,10eec <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x140>
   10ece:	0135fd63          	bgeu	a1,s3,10ee8 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x13c>
   10ed2:	00b90533          	add	a0,s2,a1
   10ed6:	00050503          	lb	a0,0(a0)
   10eda:	fc000613          	li	a2,-64
   10ede:	00c55763          	bge	a0,a2,10eec <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x140>
   10ee2:	4501                	li	a0,0
   10ee4:	e511                	bnez	a0,10ef0 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x144>
   10ee6:	a039                	j	10ef4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x148>
   10ee8:	ff359de3          	bne	a1,s3,10ee2 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x136>
   10eec:	854a                	mv	a0,s2
   10eee:	c119                	beqz	a0,10ef4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x148>
   10ef0:	89ae                	mv	s3,a1
   10ef2:	892a                	mv	s2,a0
   10ef4:	06028263          	beqz	t0,10f58 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x1ac>
   10ef8:	018a3403          	ld	s0,24(s4)
   10efc:	02000513          	li	a0,32
   10f00:	04a9f463          	bgeu	s3,a0,10f48 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x19c>
   10f04:	4501                	li	a0,0
   10f06:	00098e63          	beqz	s3,10f22 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x176>
   10f0a:	85ce                	mv	a1,s3
   10f0c:	864a                	mv	a2,s2
   10f0e:	00060683          	lb	a3,0(a2) # 110000 <_ZN8user_lib4HEAP17he3af741f2715d7f2E+0xf8ca8>
   10f12:	0605                	addi	a2,a2,1
   10f14:	fc06a693          	slti	a3,a3,-64
   10f18:	0016c693          	xori	a3,a3,1
   10f1c:	15fd                	addi	a1,a1,-1
   10f1e:	9536                	add	a0,a0,a3
   10f20:	f5fd                	bnez	a1,10f0e <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x162>
   10f22:	02857b63          	bgeu	a0,s0,10f58 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x1ac>
   10f26:	038a4583          	lbu	a1,56(s4)
   10f2a:	468d                	li	a3,3
   10f2c:	4601                	li	a2,0
   10f2e:	00d58363          	beq	a1,a3,10f34 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x188>
   10f32:	862e                	mv	a2,a1
   10f34:	00367593          	andi	a1,a2,3
   10f38:	40a40533          	sub	a0,s0,a0
   10f3c:	c1a1                	beqz	a1,10f7c <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x1d0>
   10f3e:	4605                	li	a2,1
   10f40:	04c59163          	bne	a1,a2,10f82 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x1d6>
   10f44:	4a81                	li	s5,0
   10f46:	a099                	j	10f8c <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x1e0>
   10f48:	854a                	mv	a0,s2
   10f4a:	85ce                	mv	a1,s3
   10f4c:	00000097          	auipc	ra,0x0
   10f50:	0c4080e7          	jalr	196(ra) # 11010 <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE>
   10f54:	fc8569e3          	bltu	a0,s0,10f26 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x17a>
   10f58:	008a3583          	ld	a1,8(s4)
   10f5c:	000a3503          	ld	a0,0(s4)
   10f60:	6d9c                	ld	a5,24(a1)
   10f62:	85ca                	mv	a1,s2
   10f64:	864e                	mv	a2,s3
   10f66:	60a6                	ld	ra,72(sp)
   10f68:	6406                	ld	s0,64(sp)
   10f6a:	74e2                	ld	s1,56(sp)
   10f6c:	7942                	ld	s2,48(sp)
   10f6e:	79a2                	ld	s3,40(sp)
   10f70:	7a02                	ld	s4,32(sp)
   10f72:	6ae2                	ld	s5,24(sp)
   10f74:	6b42                	ld	s6,16(sp)
   10f76:	6ba2                	ld	s7,8(sp)
   10f78:	6161                	addi	sp,sp,80
   10f7a:	8782                	jr	a5
   10f7c:	8aaa                	mv	s5,a0
   10f7e:	852e                	mv	a0,a1
   10f80:	a031                	j	10f8c <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x1e0>
   10f82:	00150593          	addi	a1,a0,1
   10f86:	8105                	srli	a0,a0,0x1
   10f88:	0015da93          	srli	s5,a1,0x1
   10f8c:	000a3b03          	ld	s6,0(s4)
   10f90:	008a3b83          	ld	s7,8(s4)
   10f94:	034a2483          	lw	s1,52(s4)
   10f98:	00150413          	addi	s0,a0,1
   10f9c:	147d                	addi	s0,s0,-1
   10f9e:	c809                	beqz	s0,10fb0 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x204>
   10fa0:	020bb603          	ld	a2,32(s7)
   10fa4:	855a                	mv	a0,s6
   10fa6:	85a6                	mv	a1,s1
   10fa8:	9602                	jalr	a2
   10faa:	d96d                	beqz	a0,10f9c <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x1f0>
   10fac:	4a05                	li	s4,1
   10fae:	a82d                	j	10fe8 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x23c>
   10fb0:	00110537          	lui	a0,0x110
   10fb4:	4a05                	li	s4,1
   10fb6:	02a48963          	beq	s1,a0,10fe8 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x23c>
   10fba:	018bb683          	ld	a3,24(s7)
   10fbe:	855a                	mv	a0,s6
   10fc0:	85ca                	mv	a1,s2
   10fc2:	864e                	mv	a2,s3
   10fc4:	9682                	jalr	a3
   10fc6:	e10d                	bnez	a0,10fe8 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x23c>
   10fc8:	4401                	li	s0,0
   10fca:	008a8c63          	beq	s5,s0,10fe2 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x236>
   10fce:	020bb603          	ld	a2,32(s7)
   10fd2:	0405                	addi	s0,s0,1
   10fd4:	855a                	mv	a0,s6
   10fd6:	85a6                	mv	a1,s1
   10fd8:	9602                	jalr	a2
   10fda:	d965                	beqz	a0,10fca <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x21e>
   10fdc:	fff40513          	addi	a0,s0,-1
   10fe0:	a011                	j	10fe4 <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E+0x238>
   10fe2:	8556                	mv	a0,s5
   10fe4:	01553a33          	sltu	s4,a0,s5
   10fe8:	8552                	mv	a0,s4
   10fea:	60a6                	ld	ra,72(sp)
   10fec:	6406                	ld	s0,64(sp)
   10fee:	74e2                	ld	s1,56(sp)
   10ff0:	7942                	ld	s2,48(sp)
   10ff2:	79a2                	ld	s3,40(sp)
   10ff4:	7a02                	ld	s4,32(sp)
   10ff6:	6ae2                	ld	s5,24(sp)
   10ff8:	6b42                	ld	s6,16(sp)
   10ffa:	6ba2                	ld	s7,8(sp)
   10ffc:	6161                	addi	sp,sp,80
   10ffe:	8082                	ret

0000000000011000 <_ZN42_$LT$str$u20$as$u20$core..fmt..Display$GT$3fmt17h34b49d6546ae1b87E>:
   11000:	86ae                	mv	a3,a1
   11002:	85aa                	mv	a1,a0
   11004:	8532                	mv	a0,a2
   11006:	8636                	mv	a2,a3
   11008:	00000317          	auipc	t1,0x0
   1100c:	da430067          	jr	-604(t1) # 10dac <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E>

0000000000011010 <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE>:
   11010:	862a                	mv	a2,a0
   11012:	051d                	addi	a0,a0,7
   11014:	ff857713          	andi	a4,a0,-8
   11018:	40c708b3          	sub	a7,a4,a2
   1101c:	0115ec63          	bltu	a1,a7,11034 <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x24>
   11020:	41158833          	sub	a6,a1,a7
   11024:	00883513          	sltiu	a0,a6,8
   11028:	0098b793          	sltiu	a5,a7,9
   1102c:	0017c793          	xori	a5,a5,1
   11030:	8d5d                	or	a0,a0,a5
   11032:	cd11                	beqz	a0,1104e <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x3e>
   11034:	4501                	li	a0,0
   11036:	c999                	beqz	a1,1104c <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x3c>
   11038:	00060683          	lb	a3,0(a2)
   1103c:	0605                	addi	a2,a2,1
   1103e:	fc06a693          	slti	a3,a3,-64
   11042:	0016c693          	xori	a3,a3,1
   11046:	15fd                	addi	a1,a1,-1
   11048:	9536                	add	a0,a0,a3
   1104a:	f5fd                	bnez	a1,11038 <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x28>
   1104c:	8082                	ret
   1104e:	00787593          	andi	a1,a6,7
   11052:	4781                	li	a5,0
   11054:	00c70f63          	beq	a4,a2,11072 <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x62>
   11058:	40e60733          	sub	a4,a2,a4
   1105c:	8532                	mv	a0,a2
   1105e:	00050683          	lb	a3,0(a0) # 110000 <_ZN8user_lib4HEAP17he3af741f2715d7f2E+0xf8ca8>
   11062:	0505                	addi	a0,a0,1
   11064:	fc06a693          	slti	a3,a3,-64
   11068:	0016c693          	xori	a3,a3,1
   1106c:	0705                	addi	a4,a4,1
   1106e:	97b6                	add	a5,a5,a3
   11070:	f77d                	bnez	a4,1105e <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x4e>
   11072:	011602b3          	add	t0,a2,a7
   11076:	4601                	li	a2,0
   11078:	cd99                	beqz	a1,11096 <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x86>
   1107a:	ff887513          	andi	a0,a6,-8
   1107e:	00a286b3          	add	a3,t0,a0
   11082:	00068503          	lb	a0,0(a3)
   11086:	0685                	addi	a3,a3,1
   11088:	fc052513          	slti	a0,a0,-64
   1108c:	00154513          	xori	a0,a0,1
   11090:	15fd                	addi	a1,a1,-1
   11092:	962a                	add	a2,a2,a0
   11094:	f5fd                	bnez	a1,11082 <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x72>
   11096:	00385713          	srli	a4,a6,0x3

000000000001109a <.LBB291_27>:
   1109a:	00002517          	auipc	a0,0x2
   1109e:	1c650513          	addi	a0,a0,454 # 13260 <.LCPI291_0>
   110a2:	00053f03          	ld	t5,0(a0)

00000000000110a6 <.LBB291_28>:
   110a6:	00002517          	auipc	a0,0x2
   110aa:	1c250513          	addi	a0,a0,450 # 13268 <.LCPI291_1>
   110ae:	00053883          	ld	a7,0(a0)
   110b2:	10001537          	lui	a0,0x10001
   110b6:	0512                	slli	a0,a0,0x4
   110b8:	0505                	addi	a0,a0,1
   110ba:	0542                	slli	a0,a0,0x10
   110bc:	00150813          	addi	a6,a0,1 # 10001001 <_ZN8user_lib4HEAP17he3af741f2715d7f2E+0xffe9ca9>
   110c0:	00f60533          	add	a0,a2,a5
   110c4:	a025                	j	110ec <.LBB291_28+0x46>
   110c6:	003e1613          	slli	a2,t3,0x3
   110ca:	00c302b3          	add	t0,t1,a2
   110ce:	41c38733          	sub	a4,t2,t3
   110d2:	003e7613          	andi	a2,t3,3
   110d6:	0115f6b3          	and	a3,a1,a7
   110da:	81a1                	srli	a1,a1,0x8
   110dc:	0115f5b3          	and	a1,a1,a7
   110e0:	95b6                	add	a1,a1,a3
   110e2:	030585b3          	mul	a1,a1,a6
   110e6:	91c1                	srli	a1,a1,0x30
   110e8:	952e                	add	a0,a0,a1
   110ea:	e241                	bnez	a2,1116a <.LBB291_28+0xc4>
   110ec:	d325                	beqz	a4,1104c <_ZN4core3str5count14do_count_chars17hf67a87501f55de2fE+0x3c>
   110ee:	83ba                	mv	t2,a4
   110f0:	8316                	mv	t1,t0
   110f2:	0c000593          	li	a1,192
   110f6:	8e3a                	mv	t3,a4
   110f8:	00b76463          	bltu	a4,a1,11100 <.LBB291_28+0x5a>
   110fc:	0c000e13          	li	t3,192
   11100:	0fce7593          	andi	a1,t3,252
   11104:	00359613          	slli	a2,a1,0x3
   11108:	00c30eb3          	add	t4,t1,a2
   1110c:	ddcd                	beqz	a1,110c6 <.LBB291_28+0x20>
   1110e:	4581                	li	a1,0
   11110:	861a                	mv	a2,t1
   11112:	da55                	beqz	a2,110c6 <.LBB291_28+0x20>
   11114:	6218                	ld	a4,0(a2)
   11116:	fff74793          	not	a5,a4
   1111a:	839d                	srli	a5,a5,0x7
   1111c:	8319                	srli	a4,a4,0x6
   1111e:	6614                	ld	a3,8(a2)
   11120:	8f5d                	or	a4,a4,a5
   11122:	01e77733          	and	a4,a4,t5
   11126:	95ba                	add	a1,a1,a4
   11128:	fff6c713          	not	a4,a3
   1112c:	831d                	srli	a4,a4,0x7
   1112e:	8299                	srli	a3,a3,0x6
   11130:	6a1c                	ld	a5,16(a2)
   11132:	8ed9                	or	a3,a3,a4
   11134:	01e6f6b3          	and	a3,a3,t5
   11138:	95b6                	add	a1,a1,a3
   1113a:	fff7c693          	not	a3,a5
   1113e:	829d                	srli	a3,a3,0x7
   11140:	0067d713          	srli	a4,a5,0x6
   11144:	6e1c                	ld	a5,24(a2)
   11146:	8ed9                	or	a3,a3,a4
   11148:	01e6f6b3          	and	a3,a3,t5
   1114c:	95b6                	add	a1,a1,a3
   1114e:	fff7c693          	not	a3,a5
   11152:	829d                	srli	a3,a3,0x7
   11154:	0067d713          	srli	a4,a5,0x6
   11158:	8ed9                	or	a3,a3,a4
   1115a:	01e6f6b3          	and	a3,a3,t5
   1115e:	02060613          	addi	a2,a2,32
   11162:	95b6                	add	a1,a1,a3
   11164:	fbd617e3          	bne	a2,t4,11112 <.LBB291_28+0x6c>
   11168:	bfb9                	j	110c6 <.LBB291_28+0x20>
   1116a:	02030a63          	beqz	t1,1119e <.LBB291_28+0xf8>
   1116e:	0c000593          	li	a1,192
   11172:	00b3e463          	bltu	t2,a1,1117a <.LBB291_28+0xd4>
   11176:	0c000393          	li	t2,192
   1117a:	4581                	li	a1,0
   1117c:	0033f613          	andi	a2,t2,3
   11180:	060e                	slli	a2,a2,0x3
   11182:	000eb683          	ld	a3,0(t4)
   11186:	0ea1                	addi	t4,t4,8
   11188:	fff6c713          	not	a4,a3
   1118c:	831d                	srli	a4,a4,0x7
   1118e:	8299                	srli	a3,a3,0x6
   11190:	8ed9                	or	a3,a3,a4
   11192:	01e6f6b3          	and	a3,a3,t5
   11196:	1661                	addi	a2,a2,-8
   11198:	95b6                	add	a1,a1,a3
   1119a:	f665                	bnez	a2,11182 <.LBB291_28+0xdc>
   1119c:	a011                	j	111a0 <.LBB291_28+0xfa>
   1119e:	4581                	li	a1,0
   111a0:	0115f633          	and	a2,a1,a7
   111a4:	81a1                	srli	a1,a1,0x8
   111a6:	0115f5b3          	and	a1,a1,a7
   111aa:	95b2                	add	a1,a1,a2
   111ac:	030585b3          	mul	a1,a1,a6
   111b0:	91c1                	srli	a1,a1,0x30
   111b2:	952e                	add	a0,a0,a1
   111b4:	8082                	ret

00000000000111b6 <_ZN4core3fmt3num3imp7fmt_u6417h46e9cec3c5125c4bE>:
   111b6:	7139                	addi	sp,sp,-64
   111b8:	fc06                	sd	ra,56(sp)
   111ba:	f822                	sd	s0,48(sp)
   111bc:	f426                	sd	s1,40(sp)
   111be:	8832                	mv	a6,a2
   111c0:	00455693          	srli	a3,a0,0x4
   111c4:	02700713          	li	a4,39
   111c8:	27100793          	li	a5,625

00000000000111cc <.LBB571_10>:
   111cc:	00001e97          	auipc	t4,0x1
   111d0:	34ce8e93          	addi	t4,t4,844 # 12518 <.Lanon.4ef9c7dd071e32d62657b33c898a56d6.287>
   111d4:	02f6f363          	bgeu	a3,a5,111fa <.LBB571_10+0x2e>
   111d8:	06300693          	li	a3,99
   111dc:	0aa6e963          	bltu	a3,a0,1128e <.LBB571_11+0x92>
   111e0:	4629                	li	a2,10
   111e2:	0ec57763          	bgeu	a0,a2,112d0 <.LBB571_11+0xd4>
   111e6:	fff70693          	addi	a3,a4,-1
   111ea:	00110613          	addi	a2,sp,1
   111ee:	9636                	add	a2,a2,a3
   111f0:	0305051b          	addiw	a0,a0,48
   111f4:	00a60023          	sb	a0,0(a2)
   111f8:	a8dd                	j	112ee <.LBB571_11+0xf2>
   111fa:	4701                	li	a4,0

00000000000111fc <.LBB571_11>:
   111fc:	00002697          	auipc	a3,0x2
   11200:	0d468693          	addi	a3,a3,212 # 132d0 <.LCPI571_0>
   11204:	0006b883          	ld	a7,0(a3)
   11208:	6689                	lui	a3,0x2
   1120a:	7106839b          	addiw	t2,a3,1808
   1120e:	6685                	lui	a3,0x1
   11210:	47b68e1b          	addiw	t3,a3,1147
   11214:	06400293          	li	t0,100
   11218:	00110313          	addi	t1,sp,1
   1121c:	05f5e6b7          	lui	a3,0x5f5e
   11220:	0ff68f1b          	addiw	t5,a3,255
   11224:	87aa                	mv	a5,a0
   11226:	03153533          	mulhu	a0,a0,a7
   1122a:	812d                	srli	a0,a0,0xb
   1122c:	0275063b          	mulw	a2,a0,t2
   11230:	40c7863b          	subw	a2,a5,a2
   11234:	03061693          	slli	a3,a2,0x30
   11238:	92c9                	srli	a3,a3,0x32
   1123a:	03c686b3          	mul	a3,a3,t3
   1123e:	0116df93          	srli	t6,a3,0x11
   11242:	82c1                	srli	a3,a3,0x10
   11244:	7fe6f413          	andi	s0,a3,2046
   11248:	025f86bb          	mulw	a3,t6,t0
   1124c:	9e15                	subw	a2,a2,a3
   1124e:	1646                	slli	a2,a2,0x31
   11250:	9241                	srli	a2,a2,0x30
   11252:	008e86b3          	add	a3,t4,s0
   11256:	00e30433          	add	s0,t1,a4
   1125a:	0006cf83          	lbu	t6,0(a3) # 5f5e000 <_ZN8user_lib4HEAP17he3af741f2715d7f2E+0x5f46ca8>
   1125e:	00168683          	lb	a3,1(a3)
   11262:	9676                	add	a2,a2,t4
   11264:	00160483          	lb	s1,1(a2)
   11268:	00064603          	lbu	a2,0(a2)
   1126c:	02d40223          	sb	a3,36(s0)
   11270:	03f401a3          	sb	t6,35(s0)
   11274:	02940323          	sb	s1,38(s0)
   11278:	02c402a3          	sb	a2,37(s0)
   1127c:	1771                	addi	a4,a4,-4
   1127e:	faff63e3          	bltu	t5,a5,11224 <.LBB571_11+0x28>
   11282:	02770713          	addi	a4,a4,39
   11286:	06300693          	li	a3,99
   1128a:	f4a6fbe3          	bgeu	a3,a0,111e0 <.LBB571_10+0x14>
   1128e:	03051613          	slli	a2,a0,0x30
   11292:	9249                	srli	a2,a2,0x32
   11294:	6685                	lui	a3,0x1
   11296:	47b6869b          	addiw	a3,a3,1147
   1129a:	02d60633          	mul	a2,a2,a3
   1129e:	8245                	srli	a2,a2,0x11
   112a0:	06400693          	li	a3,100
   112a4:	02d606bb          	mulw	a3,a2,a3
   112a8:	9d15                	subw	a0,a0,a3
   112aa:	1546                	slli	a0,a0,0x31
   112ac:	9141                	srli	a0,a0,0x30
   112ae:	1779                	addi	a4,a4,-2
   112b0:	9576                	add	a0,a0,t4
   112b2:	00150683          	lb	a3,1(a0)
   112b6:	00054503          	lbu	a0,0(a0)
   112ba:	00110793          	addi	a5,sp,1
   112be:	97ba                	add	a5,a5,a4
   112c0:	00d780a3          	sb	a3,1(a5)
   112c4:	00a78023          	sb	a0,0(a5)
   112c8:	8532                	mv	a0,a2
   112ca:	4629                	li	a2,10
   112cc:	f0c56de3          	bltu	a0,a2,111e6 <.LBB571_10+0x1a>
   112d0:	0506                	slli	a0,a0,0x1
   112d2:	ffe70693          	addi	a3,a4,-2
   112d6:	9576                	add	a0,a0,t4
   112d8:	00150603          	lb	a2,1(a0)
   112dc:	00054503          	lbu	a0,0(a0)
   112e0:	00110713          	addi	a4,sp,1
   112e4:	9736                	add	a4,a4,a3
   112e6:	00c700a3          	sb	a2,1(a4)
   112ea:	00a70023          	sb	a0,0(a4)
   112ee:	00110513          	addi	a0,sp,1
   112f2:	00d50733          	add	a4,a0,a3
   112f6:	02700513          	li	a0,39
   112fa:	40d507b3          	sub	a5,a0,a3

00000000000112fe <.LBB571_12>:
   112fe:	00001617          	auipc	a2,0x1
   11302:	17a60613          	addi	a2,a2,378 # 12478 <.Lanon.4ef9c7dd071e32d62657b33c898a56d6.2>
   11306:	8542                	mv	a0,a6
   11308:	4681                	li	a3,0
   1130a:	00000097          	auipc	ra,0x0
   1130e:	82e080e7          	jalr	-2002(ra) # 10b38 <_ZN4core3fmt9Formatter12pad_integral17hc1977205d17e7fb3E>
   11312:	70e2                	ld	ra,56(sp)
   11314:	7442                	ld	s0,48(sp)
   11316:	74a2                	ld	s1,40(sp)
   11318:	6121                	addi	sp,sp,64
   1131a:	8082                	ret

000000000001131c <_ZN4core3fmt3num3imp52_$LT$impl$u20$core..fmt..Display$u20$for$u20$u32$GT$3fmt17h1b2ac9295173ee49E>:
   1131c:	00056503          	lwu	a0,0(a0)
   11320:	862e                	mv	a2,a1
   11322:	4585                	li	a1,1
   11324:	00000317          	auipc	t1,0x0
   11328:	e9230067          	jr	-366(t1) # 111b6 <_ZN4core3fmt3num3imp7fmt_u6417h46e9cec3c5125c4bE>

000000000001132c <_ZN4core3fmt3num3imp52_$LT$impl$u20$core..fmt..Display$u20$for$u20$i64$GT$3fmt17h3dda09ee44bad10dE>:
   1132c:	6110                	ld	a2,0(a0)
   1132e:	43f65513          	srai	a0,a2,0x3f
   11332:	00a646b3          	xor	a3,a2,a0
   11336:	40a68533          	sub	a0,a3,a0
   1133a:	fff64613          	not	a2,a2
   1133e:	927d                	srli	a2,a2,0x3f
   11340:	86ae                	mv	a3,a1
   11342:	85b2                	mv	a1,a2
   11344:	8636                	mv	a2,a3
   11346:	00000317          	auipc	t1,0x0
   1134a:	e7030067          	jr	-400(t1) # 111b6 <_ZN4core3fmt3num3imp7fmt_u6417h46e9cec3c5125c4bE>

000000000001134e <_ZN4core3fmt3num3imp52_$LT$impl$u20$core..fmt..Display$u20$for$u20$u64$GT$3fmt17h637b75f18de6578eE>:
   1134e:	6108                	ld	a0,0(a0)
   11350:	862e                	mv	a2,a1
   11352:	4585                	li	a1,1
   11354:	00000317          	auipc	t1,0x0
   11358:	e6230067          	jr	-414(t1) # 111b6 <_ZN4core3fmt3num3imp7fmt_u6417h46e9cec3c5125c4bE>

000000000001135c <_ZN53_$LT$core..fmt..Error$u20$as$u20$core..fmt..Debug$GT$3fmt17h54160947d8909641E>:
   1135c:	6590                	ld	a2,8(a1)
   1135e:	6188                	ld	a0,0(a1)
   11360:	6e1c                	ld	a5,24(a2)

0000000000011362 <.LBB603_1>:
   11362:	00001597          	auipc	a1,0x1
   11366:	27e58593          	addi	a1,a1,638 # 125e0 <.Lanon.4ef9c7dd071e32d62657b33c898a56d6.631>
   1136a:	4615                	li	a2,5
   1136c:	8782                	jr	a5

000000000001136e <_ZN42_$LT$$RF$T$u20$as$u20$core..fmt..Debug$GT$3fmt17h7acd352b070da26eE>:
   1136e:	6510                	ld	a2,8(a0)
   11370:	6108                	ld	a0,0(a0)
   11372:	6e1c                	ld	a5,24(a2)
   11374:	8782                	jr	a5

0000000000011376 <_ZN44_$LT$$RF$T$u20$as$u20$core..fmt..Display$GT$3fmt17hb77f5e04fcec7777E>:
   11376:	6114                	ld	a3,0(a0)
   11378:	6510                	ld	a2,8(a0)
   1137a:	852e                	mv	a0,a1
   1137c:	85b6                	mv	a1,a3
   1137e:	00000317          	auipc	t1,0x0
   11382:	a2e30067          	jr	-1490(t1) # 10dac <_ZN4core3fmt9Formatter3pad17had8e6633ce0c6ce3E>
