plugins {
    id("com.android.application")
}

// Single source of truth for the version: the Rust workspace manifest.
val cargoVersion: String = run {
    val manifest = rootDir.resolve("../../Cargo.toml").readText()
    Regex("""\[workspace\.package][^\[]*?version\s*=\s*"([0-9.]+)""")
        .find(manifest)?.groupValues?.get(1) ?: "0.0.0"
}
val versionNumbers = cargoVersion.split(".").map { it.toIntOrNull() ?: 0 } + listOf(0, 0, 0)

// Release signing comes from the environment (CI secrets); without it, release builds are
// signed with the debug key so that CI artifacts stay installable.
val releaseKeystore: String? = System.getenv("KOEBI_ANDROID_KEYSTORE")

android {
    namespace = "io.github.kolosta.koebi"
    compileSdk = 36

    defaultConfig {
        applicationId = "io.github.kolosta.koebi"
        minSdk = 26
        targetSdk = 36
        versionName = cargoVersion
        versionCode = versionNumbers[0] * 1_000_000 + versionNumbers[1] * 1_000 + versionNumbers[2]
    }

    signingConfigs {
        create("release") {
            if (releaseKeystore != null) {
                storeFile = file(releaseKeystore)
                storePassword = System.getenv("KOEBI_ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("KOEBI_ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("KOEBI_ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName(if (releaseKeystore != null) "release" else "debug")
        }
    }

    sourceSets {
        // SDL's Java glue, copied from the locked sdl3-src crate by `cargo xtask android`.
        getByName("main").java.srcDir("src/generated/java")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }
}
