#if defined _grip_included
	#endinput
#endif
#define _grip_included

#pragma reqlib grip
#if !defined AMXMODX_NOAUTOLOAD
	#pragma loadlib grip
#endif


enum GripRequestType
{
	GripRequestTypeGet = 0,
	GripRequestTypePost = 1,
	GripRequestTypePut = 2,
	GripRequestTypeDelete = 3
}

enum GripRequest
{
	Invalid_GripRequest = 0,
}

enum GripRequestOptionsHandle {
	Invalid_GripRequestOptionsHandle = 0,
}

/**
 * Starts sending of the request 
 * @note	The handle should look like:
 * 		public RequestHandler(GripResponseHandle:handle, const userData[], const userDataSize);
 *
 *
 * @param uri		Request URI. Supports TLS.
 * @param type		Request type which should be sended.
 * @param handler	A callback which will be called when request finishes execution
 *  
 * @return		Request handle
 */
native GripRequest:grip_request(const uri[], GripRequestType:type, const handler[], GripRequestOptionsHandle:options = Invalid_GripRequestOptionsHandle, const userData[] = "", const userDataSize);

