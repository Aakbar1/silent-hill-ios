# Install on your iPhone using a Windows PC

Checked against the live English SideStore docs on **7 October 2026**. You need no
Mac or paid Apple account. This build is currently a **screen/touch test**, not the
playable game. It includes no disc image. It requires an iPhone running iOS 15+
and supports landscape only. Menu names can vary by iOS/SideStore version.

## Prepare once

1. Have a free Apple Account, the phone's passcode, a USB cable and Wi-Fi ready.
2. On the phone, install **LocalDevVPN** from the App Store. Open it, connect, and
   approve its VPN setup. It must be connected when SideStore installs or refreshes.
3. On Windows, open the [SideStore prerequisites](https://docs.sidestore.io/docs/installation/prerequisites).
   Select **Windows**, follow its iTunes download link (Apple's version is recommended),
   and install it. If the phone is not detected, the docs suggest Apple Devices.
4. On that same page click the **iloader MSI** download link. Run the downloaded
   installer and open iloader. Use the linked official downloads, not adverts.
   32-bit Windows and Windows 10 on ARM are unsupported.

These preparation steps come from the [current SideStore prerequisites](https://docs.sidestore.io/docs/installation/prerequisites).
They now name LocalDevVPN; older guides may name WireGuard or StosVPN.

## Install SideStore once

1. Connect the unlocked iPhone by USB. Tap **Trust** on the phone if asked.
2. In iloader, sign in to your Apple Account, select your phone, and click
   **Install SideStore (Stable)**. Complete authentication prompts yourself.
3. On the phone open **Settings → General → VPN & Device Management**. Select
   your Apple Account under **Developer App** and trust it. iOS 18+ uses
   **Allow & Restart**; earlier versions use **Trust**.
4. On **iOS 16+**, open **Settings → Privacy & Security → Developer Mode**.
   Turn it on, restart when asked, unlock, and confirm **Turn On**. iOS 15 has
   no Developer Mode switch. If missing, finish the SideStore install first.
5. Connect LocalDevVPN and Wi-Fi. Open SideStore and sign in with the same account.
6. Tap **My Apps**, then SideStore's days-left badge to refresh it immediately.
   If prompted to create/revoke the signing certificate, follow the refresh prompt.

Source: [SideStore installation](https://docs.sidestore.io/docs/installation/install).
See also [Apple's Developer Mode guide](https://developer.apple.com/documentation/xcode/enabling-developer-mode-on-a-device).

## Install our app

1. Ask the director for **SilentHillPort-unsigned.ipa**. A GitHub artifact download
   is an outer ZIP: on Windows right-click it → **Extract All** to find the `.ipa`.
2. Put that `.ipa` in your iCloud Drive from Windows, or use another transfer
   method that makes it visible in the iPhone **Files** app. Wait for upload.
3. On the phone connect Wi-Fi and LocalDevVPN. Open **SideStore → My Apps → +**.
   If a source menu appears, choose its file-import option. Select the `.ipa` in
   the file picker and confirm installation, then wait for signing and installation.
   Enter any Apple verification code yourself; never send credentials to the director.
4. Open **Silent Hill Port** from the Home Screen. Hold the phone sideways.
   Expect colour bars, a checked strip and orange circles under your fingers.

The IPA picker is provided by SideStore's [My Apps implementation](https://github.com/SideStore/SideStore/blob/develop/AltStore/My%20Apps/MyAppsViewController.swift).
The menu may move in future releases. No JIT setup is needed for this native app.

## Keep it working every week

Set a repeating reminder for **every 5–6 days**. While connected to Wi-Fi and
LocalDevVPN, open **SideStore → My Apps → Refresh All**. Check that both SideStore
and Silent Hill Port show about **7 DAYS** afterwards. Background refresh exists,
but check manually before the expiry date. Free accounts allow **3 active apps
including SideStore**, and **10 App IDs per week**. This shell has no extensions.
[SideStore FAQ](https://docs.sidestore.io/docs/faq).

If expired apps refuse to open, reconnect the PC and use iloader to reinstall
SideStore, then refresh/reinstall the same port app. Keep a copy of your Documents
first; deleting the app deletes its local files. If an iOS update breaks refreshing,
use the current [pairing-file instructions](https://docs.sidestore.io/docs/advanced/pairing-file):
connect by USB, replace the old stored pairing in iloader, select the phone, then
**Manage Pairing File → Place** beside SideStore. Treat pairing files as private.
For VPN/connection errors, use the [current troubleshooting page](https://docs.sidestore.io/docs/troubleshooting/common-issues).
Do not assume this October guide proves compatibility with a future iOS release.

## Put your own disc image in Files

1. Launch the app once so it creates its Documents folder and log.
2. On Windows, copy your own `.bin` (and `.cue`, if you have one) into iCloud Drive.
   Wait until the full upload finishes. The project's single-track US v1.1 image
   uses just a `.bin`; do not invent a cue file for it.
3. On the phone open **Files → Browse → iCloud Drive**. Long-press the file → **Copy**.
   Go to **Browse → On My iPhone → Silent Hill Port** and long-press empty space →
   **Paste**. Keep any cue sheet and its referenced bin together with unchanged names.
4. Confirm the file has finished copying and has the expected size. It is now
   stored for the future importer; this test shell does not read or play it yet.

If iCloud is impractical for a large image, connect by USB and use Windows
**Apple Devices → phone → Files → Silent Hill Port → Add File**. The older iTunes
equivalent is **phone icon → File Sharing → Silent Hill Port → Add**.
[Apple Devices file-sharing guide](https://support.apple.com/en-gb/120402),
[iTunes file-sharing guide](https://support.apple.com/en-gb/120403).

To send a diagnostic log, open that app folder in Files, long-press **shell.log →
Share**, and choose your usual transfer app. Back up the disc, logs and future
saves before deleting/reinstalling the app. Keep your disc image private.
