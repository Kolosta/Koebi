package io.github.kolosta.koebi;

import org.libsdl.app.SDLActivity;

/**
 * Koebi's Android entry point.
 *
 * <p>SDL's activity loads {@code libSDL3.so}, then {@code libkoebi.so} (the last library is
 * the main one) and calls its {@code SDL_main}, implemented in Rust (apps/koebi/src/lib.rs).
 */
public class KoebiActivity extends SDLActivity {
    @Override
    protected String[] getLibraries() {
        return new String[] {"SDL3", "koebi"};
    }

    /**
     * Command-line arguments for Koebi, read from the {@code koebi.args} intent extra and split
     * on whitespace, e.g.
     * {@code adb shell am start -n io.github.kolosta.koebi/.KoebiActivity --es koebi.args "'--frames 120'"}
     * (the inner quotes are for the device's shell, which adb hands the whole command line).
     */
    @Override
    protected String[] getArguments() {
        String args = getIntent() == null ? null : getIntent().getStringExtra("koebi.args");
        if (args == null || args.trim().isEmpty()) {
            return new String[0];
        }
        return args.trim().split("\\s+");
    }
}
