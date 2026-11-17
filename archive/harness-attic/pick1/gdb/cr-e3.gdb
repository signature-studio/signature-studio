set pagination off
set confirm off
set print thread-events off
set breakpoint pending on
set $Rs = (unsigned long)&R
set $Ms = (unsigned long)&main
break main
commands
silent
set $RB = $Rs + ((unsigned long)&main - $Ms)
continue
end
break send_to if ($edx==0x86||$edx==0x6||$edx==0x1c||$edx==0x21||$edx==0x7||$edx==0x8||$edx==0xa||$edx==0x18||$edx==0x2||$edx==0x82||$edx==0x10||$edx==0x1f||$edx==0x215||$edx==0x201||$edx==0x202||$edx==0xb001||$edx==0xb002||$edx==0x24||$edx==0x46||$edx==0x47)
commands
silent
printf "M %s h=%#lx msg=%#x wp=%#lx lp=%#lx\n", (char*)$rdi, 0x7f990000+4*($rsi+1), $edx, $rcx, $r8
continue
end
break do_activate if !(($rsi==136||$rsi==137) && $_streq((char*)$rdi,"ShowWindow"))
commands
silent
printf "ACT %s h=%#lx how=%lu\n", (char*)$rdi, 0x7f990000+4*($rsi+1), $rdx
continue
end
break destroy_tree
commands
silent
printf "DESTROYTREE\n"
continue
end
break fle_imp_ShowWindow
commands
silent
printf "API ShowWindow h=%#lx cmd=%ld\n", ((unsigned long*)$RB)[1], ((unsigned long*)$RB)[2]
continue
end
break fle_imp_SetFocus
commands
silent
printf "API SetFocus h=%#lx\n", ((unsigned long*)$RB)[1]
continue
end
break fle_imp_SetActiveWindow
commands
silent
printf "API SetActiveWindow h=%#lx\n", ((unsigned long*)$RB)[1]
continue
end
break fle_imp_GetActiveWindow
commands
silent
printf "API GetActiveWindow\n"
continue
end
break fle_imp_EnableWindow
commands
silent
printf "API EnableWindow h=%#lx en=%ld\n", ((unsigned long*)$RB)[1], ((unsigned long*)$RB)[2]
continue
end
break fle_imp_DestroyWindow
commands
silent
printf "API DestroyWindow h=%#lx\n", ((unsigned long*)$RB)[1]
continue
end
break fle_imp_CreateWindowExW
commands
silent
printf "API CreateWindowExW\n"
continue
end
break fle_imp_SetCapture
commands
silent
printf "API SetCapture h=%#lx\n", ((unsigned long*)$RB)[1]
continue
end
break fle_imp_ReleaseCapture
commands
silent
printf "API ReleaseCapture\n"
continue
end
break do_activate if ($rsi==136||$rsi==137) && $_streq((char*)$rdi,"ShowWindow")
commands
silent
printf "SKIPACT h=%#lx how=%lu\n", 0x7f990000+4*($rsi+1), $rdx
return
continue
end
run
