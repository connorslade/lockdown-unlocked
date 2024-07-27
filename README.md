# lockdown-unlocked

For educational purposes only.
Using DLL injection and function hooking, you can fairly easily bypass the security measures of [Respondus LockDown Browser](https://web.respondus.com/he/lockdownbrowser) without a VM or sandbox.

> Hundreds of universities and schools around the world use LockDown Browser.
> It seems that at least one person (or team) at each institution makes it a quest to “break out” or beat the system.
> Some of the best minds have taken our software to task over the years, and we’ve addressed each issue that’s been raised.
> (Yes, you have our blessing…go ahead and see if you can break it.)
>
> &ndash; [Five Little-Known Reasons to Use LockDown Browser](https://web.respondus.com/five-little-known-reasons-to-use-respondus-lockdown-browser)

## Usage

First download the latest release from the [releases page](https://github.com/connorslade/lockdown-unlocked/releases).
Inside the zip, there are three files:

- `config.toml` for defining where your LockDown Browser installation is
- `injection.dll` is the code injected into the browser process
- `launcher.exe` handles rldb links and injects browser after it starts

Just extract the zip somewhere and run the `launcher.exe` to register a handler for the rldb links. Now if you try to launch the browser from brightspace for example, it will just open as a window, not taking up the full screen, and you won’t have to close any running programs.
Note that if you move the folder to a new location, you will need to rerun the launcher as the link registration points to a specific path.
