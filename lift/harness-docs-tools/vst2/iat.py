"""usage: iat.py FILE [VA|NAME]   list the import address table of a PE (slot VA, dll!name), or look up one slot VA / one API name. Read only."""
import sys, pefile
pe = pefile.PE(sys.argv[1], fast_load=True); pe.parse_data_directories(directories=[pefile.DIRECTORY_ENTRY['IMAGE_DIRECTORY_ENTRY_IMPORT']])
base = pe.OPTIONAL_HEADER.ImageBase
q = sys.argv[2] if len(sys.argv) > 2 else None
for e in pe.DIRECTORY_ENTRY_IMPORT:
    for imp in e.imports:
        name = imp.name.decode() if imp.name else 'ord%d' % imp.ordinal
        va = imp.address
        if q is None or (q.startswith('0x') and int(q, 16) == va) or q.lower() == name.lower():
            print('%#x %s!%s' % (va, e.dll.decode().lower(), name))
