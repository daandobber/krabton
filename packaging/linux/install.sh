#!/bin/sh
# Installs Krabton for this user (no root needed): the program, a menu
# entry and the icon. Run it from the unpacked folder: ./install.sh
# Remove it again with: ./install.sh --uninstall
set -e
here=$(cd "$(dirname "$0")" && pwd)
bin="$HOME/.local/bin"
apps="$HOME/.local/share/applications"
icons="$HOME/.local/share/icons/hicolor/256x256/apps"

if [ "$1" = "--uninstall" ]; then
    rm -f "$bin/krabton" "$apps/krabton.desktop" "$icons/krabton.png"
    echo "Krabton removed."
    exit 0
fi

mkdir -p "$bin" "$apps" "$icons"
install -m 755 "$here/krabton" "$bin/krabton"
install -m 644 "$here/krabton.png" "$icons/krabton.png"
sed "s|^Exec=krabton|Exec=$bin/krabton|" "$here/krabton.desktop" > "$apps/krabton.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$apps" || true
echo "Krabton is installed: find it in your applications menu, or run $bin/krabton"
case ":$PATH:" in
    *":$bin:"*) ;;
    *) echo "(Add $bin to your PATH to start it as 'krabton' from a terminal.)" ;;
esac
