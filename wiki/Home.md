# oos-desktop

oos-desktop shows and controls an oscilloscope running [OOS](https://github.com/dekajoule/oos) from a computer with
Windows, macOS or Linux: the live waveforms of its channels, Run/Stop and Single, the settings of channels, time base,
trigger and acquisition, cursors, the scope's measurements, and screenshots and records saved as files. It talks to
the scope over the network or over a USB cable to the scope's rear port.

It is a program of its own beside the scope's web remote, which needs no installation and runs in any browser
([OOS: Remote control](https://github.com/dekajoule/oos/wiki/Remote-Control)); oos-desktop is for a large live picture
at the desk and for the USB cable when there is no network.

| | |
|---|---|
| [[Installation]] | build and start it on Windows, macOS or Linux |
| [[Connecting]] | find the scope on the network or on USB, what the scope needs switched on |
| [[The Window|The-Window]] | the waveform, cursors, the strip, the panels, the keys |
| [[Troubleshooting]] | when Find finds nothing, a value is refused, or USB does not show |

**State:** the program has run against a DHO924S with OOS over the network and over USB from a Mac, and in CI on
Windows, macOS and Linux; USB from Windows and Linux is not yet tried with this program. There are no ready-made
downloads yet: it is built from its source ([[Installation]]).
