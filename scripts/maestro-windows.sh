#!/usr/bin/env bash
# Maestro on the Windows side of a WSL box. WSL cannot reach the Windows adb
# server: even started with `-a` it sits behind the Windows firewall, and
# there is no admin on this machine to open it. So Maestro runs where adb
# does. `C:\Users\<user>\dz-maestro.cmd` sets JAVA_HOME to the Windows JDK 17
# and calls maestro.bat; DZPOS_MAESTRO_CMD points at another copy.
exec cmd.exe /c "${DZPOS_MAESTRO_CMD:-C:\\Users\\Anwender\\dz-maestro.cmd}" "$@"
