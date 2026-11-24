# Changelog

What a user notices, newest first.

## Unreleased

- A new look: a top bar with Run/Stop in the scope's colours, the trigger's state and the tools; a strip of channel,
  time base, acquisition and trigger cards (click switches a channel, the wheel turns its knob); the measurements below
  the waveform; a start page that finds scopes; a status line with the records per second; a window icon.
- The waveform is placed as on the scope's screen, by V/div, offset, s/div and delay, with the time of each division,
  markers of ground, trigger level and trigger time, and the values under the pointer.
- Time and level cursors with ΔT, 1/ΔT and ΔV.
- More settings: probe, bandwidth limit and invert of a channel; expand point and auto roll of the time base;
  coupling, holdoff and noise reject of the trigger; a new Acquire panel with mode and averages.
- Auto (autoscale), Clear and Force; keys Space, S, F and C; an address on the command line connects at the start.
- Faster: settings, measurements and the acquisition's state are read in one round trip each, and only the records of
  the channels that are on; against `oos-ui --serve` about eight times as many records a second.
- USB: a scope on a cable to its rear port (USBTMC), found by Find; needs OOS with its USB instrument.
- Find: the scopes on the network, also on a cable straight to the computer, to connect with one click.
- Screenshot as PNG and the records of the channels that are on as CSV, saved where you choose.
- Measure: the scope's measurements of the channels that are on (peak-peak, frequency and eight more to choose).
- A first window: connect to an OOS scope over the network, Run/Stop/Single, the records of its channels, and the
  settings of channels, time base and trigger.
