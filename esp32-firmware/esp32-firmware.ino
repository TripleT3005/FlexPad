#include <Arduino.h>
#include <FS.h>
#include <LittleFS.h>
#include <ArduinoJson.h>
#include "USB.h"
#include "USBHIDKeyboard.h"
#include "USBHIDConsumerControl.h"

USBHIDKeyboard Keyboard;
USBHIDConsumerControl ConsumerControl;

const int monitoredPins[] = {
  1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 
  21, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 47, 48
};
const int numPins = sizeof(monitoredPins) / sizeof(monitoredPins[0]);

struct PinState {
  int rawState;
  int debouncedState;
  unsigned long lastDebounceTime;
  unsigned long lastTriggerTime;
};

PinState pinStates[numPins];
String pinSequences[50]; 
const unsigned long debounceDelay = 50;
const unsigned long cooldownTime = 100;

void loadConfiguration() {
  for(int i = 0; i < 50; i++) pinSequences[i] = "";

  if (!LittleFS.exists("/config.json")) return;

  File configFile = LittleFS.open("/config.json", "r");
  if (!configFile) return;
  
  JsonDocument doc;
  if (deserializeJson(doc, configFile)) {
    configFile.close();
    return;
  }
  configFile.close();

  JsonObject pinsObj = doc["pins"];
  for (JsonPair kv : pinsObj) {
    int pinNum = atoi(kv.key().c_str());
    if (kv.value().is<String>()) {
      pinSequences[pinNum] = kv.value().as<String>();
    }
  }
}

int parseKeyCode(String keyName) {
  keyName.trim();
  keyName.toLowerCase();
  
  if (keyName == "ctrl" || keyName == "lctrl") return KEY_LEFT_CTRL;
  if (keyName == "rctrl") return KEY_RIGHT_CTRL;
  if (keyName == "shift" || keyName == "lshift") return KEY_LEFT_SHIFT;
  if (keyName == "rshift") return KEY_RIGHT_SHIFT;
  if (keyName == "alt" || keyName == "lalt") return KEY_LEFT_ALT;
  if (keyName == "altgr" || keyName == "ralt") return KEY_RIGHT_ALT;
  if (keyName == "win" || keyName == "gui" || keyName == "lwin") return KEY_LEFT_GUI;
  if (keyName == "rwin") return KEY_RIGHT_GUI;
  
  if (keyName == "enter" || keyName == "return") return KEY_RETURN;
  if (keyName == "esc" || keyName == "escape") return KEY_ESC;
  if (keyName == "backspace") return KEY_BACKSPACE;
  if (keyName == "del" || keyName == "delete") return KEY_DELETE;
  if (keyName == "tab") return KEY_TAB;
  if (keyName == "space") return ' ';
  if (keyName == "up") return KEY_UP_ARROW;
  if (keyName == "down") return KEY_DOWN_ARROW;
  if (keyName == "left") return KEY_LEFT_ARROW;
  if (keyName == "right") return KEY_RIGHT_ARROW;
  
  if (keyName.startsWith("f") && keyName.length() <= 3) {
    int fNum = keyName.substring(1).toInt();
    switch (fNum) {
      case 1: return KEY_F1;   case 2: return KEY_F2;   case 3: return KEY_F3;   case 4: return KEY_F4;
      case 5: return KEY_F5;   case 6: return KEY_F6;   case 7: return KEY_F7;   case 8: return KEY_F8;
      case 9: return KEY_F9;   case 10: return KEY_F10; case 11: return KEY_F11; case 12: return KEY_F12;
      case 13: return KEY_F13; case 14: return KEY_F14; case 15: return KEY_F15; case 16: return KEY_F16;
      case 17: return KEY_F17; case 18: return KEY_F18; case 19: return KEY_F19; case 20: return KEY_F20;
      case 21: return KEY_F21; case 22: return KEY_F22; case 23: return KEY_F23; case 24: return KEY_F24;
    }
  }

  if (keyName == "quote" || keyName == "'") return '\'';
  if (keyName == "semicolon" || keyName == ";") return ';';
  if (keyName == "bracket_left" || keyName == "[") return '[';
  if (keyName == "minus" || keyName == "-") return '-';

  if (keyName.length() == 1) return keyName[0];
  return 0;
}

void executeSequence(int pin) {
  String seq = pinSequences[pin];
  
  if (seq.length() == 0) {
    String fallback = "FlexPad unassigned pin " + String(pin);
    for (unsigned int c = 0; c < fallback.length(); c++) {
      Keyboard.write(fallback[c]);
      delay(2);
    }
    return;
  }

  int startIndex = 0;
  while (startIndex < (int)seq.length()) {
    int commaIndex = seq.indexOf(',', startIndex);
    if (commaIndex == -1) commaIndex = seq.length();
    
    String token = seq.substring(startIndex, commaIndex);
    token.trim();

    if (token.startsWith("P:")) {
      int key = parseKeyCode(token.substring(2));
      if (key != 0) Keyboard.press(key);
    } 
    else if (token.startsWith("R:")) {
      int key = parseKeyCode(token.substring(2));
      if (key != 0) Keyboard.release(key);
    } 
    else if (token.startsWith("D:")) {
      int ms = token.substring(2).toInt();
      delay(ms);
    }
    else if (token.startsWith("MEDIA:")) {
      String mAction = token.substring(6);
      mAction.toLowerCase();
      if (mAction == "vol_up") { ConsumerControl.press(CONSUMER_CONTROL_VOLUME_INCREMENT); delay(30); ConsumerControl.release(); }
      else if (mAction == "vol_down") { ConsumerControl.press(CONSUMER_CONTROL_VOLUME_DECREMENT); delay(30); ConsumerControl.release(); }
      else if (mAction == "mute") { ConsumerControl.press(CONSUMER_CONTROL_MUTE); delay(30); ConsumerControl.release(); }
      else if (mAction == "next") { ConsumerControl.press(CONSUMER_CONTROL_SCAN_NEXT); delay(30); ConsumerControl.release(); }
      else if (mAction == "prev") { ConsumerControl.press(CONSUMER_CONTROL_SCAN_PREVIOUS); delay(30); ConsumerControl.release(); }
      else if (mAction == "play") { ConsumerControl.press(CONSUMER_CONTROL_PLAY_PAUSE); delay(30); ConsumerControl.release(); }
    }

    startIndex = commaIndex + 1;
  }
}

String deviceSerial = "";

void setup() {
  uint64_t chipid = ESP.getEfuseMac();
  char serialBuffer[20];
  snprintf(serialBuffer, sizeof(serialBuffer), "FlexPad-%04X%08X", (uint16_t)(chipid >> 32), (uint32_t)chipid);
  deviceSerial = String(serialBuffer);

  USB.productName("FlexPad");
  USB.serialNumber(deviceSerial.c_str());
  
  USB.begin();
  Keyboard.begin();
  ConsumerControl.begin();
  Serial.begin(115200);
  delay(2000); 

  if (LittleFS.begin(true, "/littlefs", 10, "storage")) {
    loadConfiguration();
  }

  for (int i = 0; i < numPins; i++) {
    pinMode(monitoredPins[i], INPUT_PULLUP);
    pinStates[i].rawState = digitalRead(monitoredPins[i]);
    pinStates[i].debouncedState = pinStates[i].rawState;
    pinStates[i].lastDebounceTime = 0;
    pinStates[i].lastTriggerTime = 0;
  }
  
  Serial.println("FlexPad ready.");
}

void loop() {
  if (Serial.available()) {
    String input = Serial.readStringUntil('\n');
    input.trim();
    
    if (input.startsWith("SET_CONFIG:")) {
      File f = LittleFS.open("/config.json", "w");
      if (f) {
        f.print(input.substring(11));
        f.close();
        loadConfiguration();
        Serial.println("CONFIG_SAVED");
      } else {
        Serial.println("ERROR_FILE_OPEN");
      }
    }
    else if (input.equals("GET_CONFIG")) {
      if (LittleFS.exists("/config.json")) {
        File f = LittleFS.open("/config.json", "r");
        if (f) {
          Serial.print("CONFIG_DATA:");
          while (f.available()) {
            Serial.write(f.read());
          }
          Serial.println();
          f.close();
        }
      } else {
        Serial.println("CONFIG_DATA:{}");
      }
    }
    else if (input.equals("GET_DEVICE_INFO")) {
      Serial.printf("DEVICE_INFO:FlexPad,%s\n", deviceSerial.c_str());
    }
  }

  unsigned long currentMillis = millis();
  for (int i = 0; i < numPins; i++) {
    int pin = monitoredPins[i];
    int reading = digitalRead(pin);

    if (reading != pinStates[i].rawState) {
      pinStates[i].rawState = reading;
      pinStates[i].lastDebounceTime = currentMillis;
    }

    if ((currentMillis - pinStates[i].lastDebounceTime) > debounceDelay) {
      if (pinStates[i].rawState != pinStates[i].debouncedState) {
        pinStates[i].debouncedState = pinStates[i].rawState;

        if (pinStates[i].debouncedState == LOW) {
          if (currentMillis - pinStates[i].lastTriggerTime > cooldownTime) {
            pinStates[i].lastTriggerTime = currentMillis;
            executeSequence(pin);
          }
        }
      }
    }
  }
}