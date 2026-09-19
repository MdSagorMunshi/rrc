plugins {
    kotlin("jvm") version "1.9.22"
}

group = "com.github.mdsagormunshi.rrc"
version = "0.1.0"

repositories {
    mavenCentral()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.8.0")
    testImplementation(kotlin("test"))
}

tasks.test {
    useJUnitPlatform()
}

// UniFFI generated bindings Gradle helper task
tasks.register<Exec>("generateUniFFIBindings") {
    workingDir = projectDir
    commandLine(
        "cargo", "run", "-p", "uniffi-bindgen", "--",
        "generate", "src/rrc.udl",
        "--language", "kotlin",
        "--out-dir", "src/main/kotlin"
    )
}
