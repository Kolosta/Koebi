// Gradle project packaging Koebi for Android. Build it with `cargo xtask android`, which
// compiles libkoebi.so / libSDL3.so and copies SDL's Java glue before running Gradle.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "Koebi"
include(":app")
