# sdboot-oneshot

An attempt to make [`bootctl set-oneshot
...`](https://man7.org/linux/man-pages/man1/bootctl.1.html) available on both
linux and windows.

Effectively the app is only capable of listing boot entries (`LoaderEntries` EFI
variable) and updating the oneshot entry (`LoaderEntryOneShot` EFI variable).

Setting the oneshot entry requires root privileges.
