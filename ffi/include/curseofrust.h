#ifndef CURSEOFRUST_H
#define CURSEOFRUST_H

typedef struct {
  void *first;
  void *second;
} CORParseOptionsReturn;

typedef void *CORBasicOptsRef;
typedef void *CORMultiplayerOptsRef;
typedef void *CORStateRef;
typedef void *CORUIRef;

CORParseOptionsReturn CORParseOptions(char *optStringPtr);
void CORReleaseErrorString(char *errorStringPtr);
void *CORMakeState(CORBasicOptsRef basicOptsPtr);
CORUIRef CORMakeUI(CORStateRef statePtr);

#endif